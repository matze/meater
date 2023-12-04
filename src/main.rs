use anyhow::Context;
use embedded_graphics::geometry::Point;
use embedded_graphics::image::Image;
use embedded_graphics::mono_font::MonoTextStyle;
use embedded_graphics::pixelcolor::BinaryColor;
use embedded_graphics::text::Text;
use embedded_graphics::Drawable;
use profont::{PROFONT_24_POINT, PROFONT_7_POINT};

mod meater;

#[tokio::main(flavor = "current_thread")]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let interface = rppal::i2c::I2c::new().context("unable to create I2c")?;

    let mut display: sh1106::mode::GraphicsMode<_> = sh1106::Builder::new()
        .with_size(sh1106::displaysize::DisplaySize::Display128x64)
        .connect_i2c(interface)
        .into();

    let not_found_icon = tinybmp::Bmp::from_slice(include_bytes!("assets/not-found.bmp")).unwrap();
    let connecting_icon =
        tinybmp::Bmp::from_slice(include_bytes!("assets/connecting.bmp")).unwrap();
    let battery_icon_25 =
        tinybmp::Bmp::from_slice(include_bytes!("assets/battery-25.bmp")).unwrap();
    let battery_icon_50 =
        tinybmp::Bmp::from_slice(include_bytes!("assets/battery-50.bmp")).unwrap();
    let battery_icon_75 =
        tinybmp::Bmp::from_slice(include_bytes!("assets/battery-75.bmp")).unwrap();
    let battery_icon_100 =
        tinybmp::Bmp::from_slice(include_bytes!("assets/battery-100.bmp")).unwrap();

    display.init().unwrap();
    display.flush().unwrap();
    display.clear();

    Image::new(&not_found_icon, Point::new(47, 16)).draw(&mut display)?;

    display.flush().unwrap();

    let temperature_style = MonoTextStyle::new(&PROFONT_24_POINT, BinaryColor::On);
    let mut temperature: Option<(f32, f32)> = None;
    let mut battery: Option<u16> = None;

    let (client, mut receiver) = meater::Client::new();

    let event_handling = async move {
        let mut state = meater::State::Disconnected;

        while let Some(event) = receiver.recv().await {
            match event {
                meater::Event::State(new_state) => state = new_state,
                meater::Event::Temperature { tip, ambient } => {
                    temperature.replace((tip, ambient));
                }
                meater::Event::Battery { percent } => {
                    battery.replace(percent);
                }
            }

            display.clear();

            match state {
                meater::State::Disconnected => {
                    Image::new(&not_found_icon, Point::new(47, 16)).draw(&mut display)?;
                }
                meater::State::Connecting => {
                    Image::new(&connecting_icon, Point::new(47, 16)).draw(&mut display)?;
                }
                meater::State::Connected => {
                    if let Some((tip, _ambient)) = temperature {
                        Text::new(&format!("{tip:.0}°C"), Point::new(0, 38), temperature_style)
                            .draw(&mut display)?;
                    }

                    if let Some(percent) = battery {
                        let icon = match percent as u16 {
                            ..=25 => battery_icon_25,
                            26..=50 => battery_icon_50,
                            51..=75 => battery_icon_75,
                            _ => battery_icon_100,
                        };

                        Image::new(&icon, Point::new(112, 0)).draw(&mut display)?;
                    }
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
