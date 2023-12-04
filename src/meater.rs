use anyhow::anyhow;
use btleplug::api::{
    Central, CentralEvent, CharPropFlags, Manager, Peripheral, ScanFilter, ValueNotification,
};
use btleplug::platform;
use futures::StreamExt;
use tokio::sync::mpsc;
use uuid::uuid;

const SERVICE_UUID: uuid::Uuid = uuid!("a75cc7fc-c956-488f-ac2a-2dbc08b63a04");
const BATTERY_UUID: uuid::Uuid = uuid!("2adb4877-68d8-4884-bd3c-d83853bf27b8");
const TEMPERATURE_UUID: uuid::Uuid = uuid!("7edda774-045e-4bbf-909b-45d1991a2876");

/// State the MEATER device may be in.
pub enum State {
    Disconnected,
    Connecting,
    Connected,
}

/// An event emitted by the MEATER client.
pub enum Event {
    /// State changed.
    State(State),
    /// Temperature changed.
    Temperature { tip: f32, ambient: f32 },
    /// Battery level changed.
    Battery { percent: u16 },
}

pub struct Client(mpsc::Sender<Event>);

impl Client {
    pub fn new() -> (Self, mpsc::Receiver<Event>) {
        let (sender, receiver) = mpsc::channel(16);
        (Self(sender), receiver)
    }

    pub async fn run(self) -> anyhow::Result<()> {
        self.0.send(Event::State(State::Disconnected)).await?;

        let manager = platform::Manager::new().await?;

        // This sometimes fails as well ...
        let central = manager
            .adapters()
            .await?
            .into_iter()
            .nth(0)
            .ok_or(anyhow!("no bluetooth adapter found"))?;

        monitor(&central, self.0).await?;

        Ok(())
    }
}

/// Return `Ok(Some(meater))` if `id` is a MEATER device.
async fn get_meater(
    central: &platform::Adapter,
    id: &platform::PeripheralId,
) -> anyhow::Result<Option<platform::Peripheral>> {
    let peripheral = central.peripheral(id).await?;

    Ok(peripheral
        .properties()
        .await?
        .and_then(|props| props.local_name)
        .map(|name| name == "MEATER")
        .unwrap_or_default()
        .then_some(peripheral))
}

/// Connect to the meater and subscribe to all notification characteristics.
async fn connect(meater: &platform::Peripheral) -> anyhow::Result<()> {
    meater.connect().await?;
    meater.discover_services().await?;

    for characteristic in meater.characteristics() {
        if characteristic.properties.contains(CharPropFlags::NOTIFY) {
            tracing::debug!(characteristic = ?characteristic, "subscribing");
            meater.subscribe(&characteristic).await?;
        }
    }

    Ok(())
}

/// Listen to notifications and send out temperature and battery values.
async fn listen(meater: platform::Peripheral, sender: mpsc::Sender<Event>) -> anyhow::Result<()> {
    let mut notifications = meater.notifications().await?;

    while let Some(ValueNotification { value, uuid }) = notifications.next().await {
        tracing::info!(uuid = ?uuid, value = ?value, "received notification value");

        if uuid == TEMPERATURE_UUID {
            if value.len() != 8 {
                tracing::warn!("temperature does not contain correct number of bytes");
                continue;
            }

            let tip = to_u16(value[1], value[0]);
            let ra = to_u16(value[3], value[2]);
            let oa = to_u16(value[5], value[4]);
            let ambient = tip + 0.max(((ra - 48.min(oa)) * 16 * 589) / 1487);

            sender
                .send(Event::Temperature {
                    tip: to_degree_celsius(tip),
                    ambient: to_degree_celsius(ambient),
                })
                .await?;
        } else if uuid == BATTERY_UUID {
            sender
                .send(Event::Battery {
                    percent: to_u16(value[1], value[0]) * 10,
                })
                .await?;
        }
    }

    Ok(())
}

/// Start main event loop handling state changes between discovery, connection and connection loss.
async fn monitor(
    central: &platform::Adapter,
    sender: mpsc::Sender<Event>,
) -> anyhow::Result<platform::Peripheral> {
    tracing::info!("looking for MEATER device");

    let mut events = central.events().await?;

    central
        .start_scan(ScanFilter {
            services: vec![SERVICE_UUID],
        })
        .await?;

    let mut current_listener = None;

    while let Some(event) = events.next().await {
        match event {
            CentralEvent::DeviceDiscovered(id) => {
                if let Some(meater) = get_meater(central, &id).await? {
                    tracing::info!(id = ?id, "MEATER discovered");
                    sender.send(Event::State(State::Connecting)).await?;
                    connect(&meater).await?;
                    tokio::spawn(listen(meater, sender.clone()));
                }
            }
            CentralEvent::DeviceConnected(id) => {
                if let Some(meater) = get_meater(central, &id).await? {
                    // if get_meater(&central, &id).await?.is_some() {
                    tracing::info!(id = ?id, "MEATER connected");
                    sender.send(Event::State(State::Connected)).await?;
                    current_listener.replace(tokio::spawn(listen(meater, sender.clone())));
                }
            }
            CentralEvent::DeviceDisconnected(id) => {
                if get_meater(central, &id).await?.is_some() {
                    tracing::info!(id = ?id, "MEATER disconnected");
                    sender.send(Event::State(State::Disconnected)).await?;

                    if let Some(listener) = current_listener.take() {
                        listener.abort();
                    }
                }
            }
            CentralEvent::DeviceUpdated(id) => {
                if let Some(meater) = get_meater(central, &id).await? {
                    tracing::info!(id = ?id, "MEATER updated");
                    sender.send(Event::State(State::Connecting)).await?;
                    connect(&meater).await?;
                }
            }
            _ => {}
        }
    }

    Err(anyhow!("no meater found"))
}

fn to_u16(msb: u8, lsb: u8) -> u16 {
    u16::from(msb) * 256 + u16::from(lsb)
}

fn to_degree_celsius(value: u16) -> f32 {
    (f32::from(value) + 8.0) / 16.0
}
