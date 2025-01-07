use anyhow::{anyhow, Context};
use embedded_graphics::geometry::Point;
use embedded_graphics::image::Image;
use embedded_graphics::mono_font::MonoTextStyle;
use embedded_graphics::pixelcolor::BinaryColor;
use embedded_graphics::text::Text;
use embedded_graphics::Drawable;
use futures::{stream, Stream, StreamExt};
use profont::{PROFONT_24_POINT, PROFONT_9_POINT};
use tokio::sync::mpsc;

mod icons;
mod meater;

/// Consolidated MEATER state
#[derive(Clone, Debug)]
struct MeaterState {
    tip: f32,
    ambient: f32,
    percentage: u16,
}

/// Consolidate events.
enum Event {
    /// Show centered icon.
    Icon(tinybmp::Bmp<'static, BinaryColor>),
    /// Show state update.
    Update(MeaterState),
}

/// Turn [`meater::Event`]s into consolidate state [`Event`]s.
fn process_events(receiver: mpsc::Receiver<meater::Event>) -> impl Stream<Item = Event> {
    struct State {
        receiver: mpsc::Receiver<meater::Event>,
        state: MeaterState,
    }

    let stream_state = State {
        receiver,
        state: MeaterState {
            tip: 20.0,
            ambient: 20.0,
            percentage: 100,
        },
    };

    stream::unfold(stream_state, |mut stream_state| async move {
        loop {
            if let Some(event) = stream_state.receiver.recv().await {
                match event {
                    meater::Event::State(state) => match state {
                        meater::State::Disconnected => {
                            break Some((Event::Icon(icons::DISCONNECTED), stream_state))
                        }
                        meater::State::Connecting => {
                            break Some((Event::Icon(icons::CONNECTING), stream_state))
                        }
                    },
                    meater::Event::Temperature { tip, ambient } => {
                        stream_state.state.tip = tip;
                        stream_state.state.ambient = ambient;
                        break Some((Event::Update(stream_state.state.clone()), stream_state));
                    }
                    meater::Event::Battery { percent } => {
                        stream_state.state.percentage = percent;
                        break Some((Event::Update(stream_state.state.clone()), stream_state));
                    }
                }
            } else {
                break None;
            }
        }
    })
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let i2c = rppal::i2c::I2c::new().context("unable to create I2c")?;

    let mut display: sh1106::mode::GraphicsMode<_> = sh1106::builder::Builder::new()
        .with_size(sh1106::displaysize::DisplaySize::Display128x64)
        .connect_i2c(i2c)
        .into();

    display
        .init()
        .map_err(|err| anyhow!("failed to init display: {err:?}"))?;

    display.clear();

    Image::new(&icons::DISCONNECTED, Point::new(47, 16))
        .draw(&mut display)
        .unwrap();

    display.flush().unwrap();

    let temperature_style = MonoTextStyle::new(&PROFONT_24_POINT, BinaryColor::On);
    let description_style = MonoTextStyle::new(&PROFONT_9_POINT, BinaryColor::On);

    let (client, receiver) = meater::Client::new();

    let event_handling = async move {
        let mut stream = std::pin::pin!(process_events(receiver));

        while let Some(event) = stream.next().await {
            display.clear();

            match event {
                Event::Icon(icon) => {
                    Image::new(&icon, Point::new(47, 16))
                        .draw(&mut display)
                        .unwrap();
                }
                Event::Update(MeaterState {
                    tip,
                    ambient,
                    percentage,
                }) => {
                    Text::new(&format!("{tip:.0}"), Point::new(0, 28), temperature_style)
                        .draw(&mut display)
                        .unwrap();

                    Text::new(&format!("tip"), Point::new(34, 27), description_style)
                        .draw(&mut display)
                        .unwrap();

                    Text::new(
                        &format!("{ambient:.0}"),
                        Point::new(0, 60),
                        temperature_style,
                    )
                    .draw(&mut display)
                    .unwrap();

                    Text::new(&format!("ambient"), Point::new(34, 59), description_style)
                        .draw(&mut display)
                        .unwrap();

                    let icon = match percentage {
                        ..=25 => icons::BATTERY_25,
                        26..=50 => icons::BATTERY_50,
                        51..=75 => icons::BATTERY_75,
                        _ => icons::BATTERY_100,
                    };

                    Image::new(&icon, Point::new(112, 0))
                        .draw(&mut display)
                        .unwrap();
                }
            }

            display.flush().unwrap();
        }

        Ok::<_, anyhow::Error>(())
    };

    let result = tokio::join!(client.run(), event_handling);

    result.0?;
    result.1?;

    Ok(())
}
