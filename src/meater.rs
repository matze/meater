use anyhow::anyhow;
use btleplug::api::{
    Central, CentralEvent, CharPropFlags, Manager, Peripheral, ScanFilter, ValueNotification,
};
use btleplug::platform;
use futures::StreamExt;
use tokio::sync::mpsc;
use uuid::uuid;

const BATTERY_UUID: uuid::Uuid = uuid!("2adb4877-68d8-4884-bd3c-d83853bf27b8");
const TEMPERATURE_UUID: uuid::Uuid = uuid!("7edda774-045e-4bbf-909b-45d1991a2876");

pub enum Event {
    Message(&'static str),
    Temperature { tip: f32, ambient: f32 },
    Battery { percent: u16 },
}

pub struct Client(mpsc::Sender<Event>);

impl Client {
    pub fn new() -> (Self, mpsc::Receiver<Event>) {
        let (sender, receiver) = mpsc::channel(16);
        (Self(sender), receiver)
    }

    pub async fn run(self) -> anyhow::Result<()> {
        self.0
            .send(Event::Message("Connecting bluetooth ..."))
            .await?;
        let manager = platform::Manager::new().await?;

        let central = manager
            .adapters()
            .await?
            .into_iter()
            .nth(0)
            .ok_or(anyhow!("no bluetooth adapter found"))?;

        self.0.send(Event::Message("Finding MEATER ...")).await?;
        let meater = find_meater(&central).await?;

        self.0.send(Event::Message("Connecting MEATER ...")).await?;
        meater.connect().await?;
        meater.discover_services().await?;

        for characteristic in meater.characteristics() {
            if characteristic.properties.contains(CharPropFlags::NOTIFY) {
                tracing::debug!(characteristic = ?characteristic, "subscribing");
                meater.subscribe(&characteristic).await?;
            }
        }

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

                self.0
                    .send(Event::Temperature {
                        tip: to_degree_celsius(tip),
                        ambient: to_degree_celsius(ambient),
                    })
                    .await?;
            } else if uuid == BATTERY_UUID {
                self.0
                    .send(Event::Battery {
                        percent: to_u16(value[1], value[0]) * 10,
                    })
                    .await?;
            }
        }

        Ok(())
    }
}

async fn find_meater(central: &platform::Adapter) -> anyhow::Result<platform::Peripheral> {
    tracing::info!("looking for MEATER device");

    let mut events = central.events().await?;
    central.start_scan(ScanFilter::default()).await?;

    while let Some(event) = events.next().await {
        if let CentralEvent::DeviceDiscovered(id) = event {
            let peripheral = central.peripheral(&id).await?;

            if let Some(props) = peripheral.properties().await? {
                if let Some(name) = &props.local_name {
                    if name == "MEATER" {
                        tracing::debug!(peripheral = ?peripheral, "found device");
                        return Ok(peripheral);
                    }
                }
            }
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
