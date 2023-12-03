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

    display.init().unwrap();
    display.flush().unwrap();

    let message_style = MonoTextStyle::new(&PROFONT_7_POINT, BinaryColor::On);
    let temperature_style = MonoTextStyle::new(&PROFONT_24_POINT, BinaryColor::On);

    let (client, mut receiver) = meater::Client::new();

    let mut message: Option<&'static str> = None;
    let mut temperature: Option<(f32, f32)> = None;
    let mut battery: Option<u16> = None;

    let battery_icons: [tinybmp::Bmp<'static, BinaryColor>; 4] = [
        tinybmp::Bmp::from_slice(include_bytes!("assets/battery-25.bmp")).unwrap(),
        tinybmp::Bmp::from_slice(include_bytes!("assets/battery-50.bmp")).unwrap(),
        tinybmp::Bmp::from_slice(include_bytes!("assets/battery-75.bmp")).unwrap(),
        tinybmp::Bmp::from_slice(include_bytes!("assets/battery-100.bmp")).unwrap(),
    ];

    let event_handling = async move {
        while let Some(event) = receiver.recv().await {
            display.clear();

            match event {
                meater::Event::Message(new_message) => {
                    message.replace(new_message);
                }
                meater::Event::Temperature { tip, ambient } => {
                    temperature.replace((tip, ambient));
                }
                meater::Event::Battery { percent } => {
                    battery.replace(percent);
                }
            }

            if let Some(message) = message.take() {
                Text::new(message, Point::new(10, 30), message_style).draw(&mut display)?;
            } else {
                if let Some((tip, _ambient)) = temperature {
                    Text::new(&format!("{tip:.0}°C"), Point::new(0, 38), temperature_style)
                        .draw(&mut display)?;
                }

                if let Some(percent) = battery {
                    let icon = match percent as u16 {
                        ..=25 => battery_icons[0],
                        26..=50 => battery_icons[1],
                        51..=75 => battery_icons[2],
                        _ => battery_icons[3],
                    };

                    Image::new(&icon, Point::new(112, 0)).draw(&mut display)?;
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
