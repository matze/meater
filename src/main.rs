use anyhow::{anyhow, Context};
use embedded_graphics::image::{Image, SubImage};
use embedded_graphics::pixelcolor::BinaryColor;
use embedded_graphics::primitives::Rectangle;
use embedded_graphics::Drawable;
use embedded_graphics::{
    geometry::{OriginDimensions, Point, Size},
    image::ImageDrawableExt,
};
use futures::{stream, Stream, StreamExt};
use tokio::sync::mpsc;

mod icons;
mod meater;

/// Consolidate events.
enum Event {
    /// Show centered icon.
    Icon(tinybmp::Bmp<'static, BinaryColor>),
    /// Show temperature.
    Update(f32),
}

/// Turn [`meater::Event`]s into consolidate state [`Event`]s.
fn process_events(receiver: mpsc::Receiver<meater::Event>) -> impl Stream<Item = Event> {
    struct State {
        receiver: mpsc::Receiver<meater::Event>,
        temperature: f32,
    }

    let stream_state = State {
        receiver,
        temperature: 20.0,
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
                    meater::Event::Temperature { tip, ambient: _ } => {
                        stream_state.temperature = tip;
                        break Some((Event::Update(tip), stream_state));
                    }
                    _ => {}
                }
            } else {
                break None;
            }
        }
    })
}

fn draw_number<T: sh1106::interface::DisplayInterface>(
    value: f32,
    glyphs: &[SubImage<'_, tinybmp::Bmp<BinaryColor>>],
    display: &mut sh1106::mode::GraphicsMode<T>,
) {
    tracing::info!(value, "computed");
    let value = 99.0_f32.min(value);
    let i1 = (value as usize) / 10;
    let i2 = (value as usize) - (i1 * 10);
    let i3 = ((value * 10.0) % 10.0) as usize;

    let n1 = &glyphs[i1];
    let n2 = &glyphs[i2];
    let period = &glyphs[10];
    let n3 = &glyphs[i3];

    let mut x = 0;
    Image::new(n1, Point::new(x, 0)).draw(display).unwrap();
    x += n1.size().width as i32;
    Image::new(n2, Point::new(x, 0)).draw(display).unwrap();
    // We shift the period back a bit for tighter looks.
    x += n2.size().width as i32 - 2;
    Image::new(period, Point::new(x, 0)).draw(display).unwrap();
    x += period.size().width as i32;
    Image::new(n3, Point::new(x, 0)).draw(display).unwrap();
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let i2c = rppal::i2c::I2c::new().context("unable to create I2c")?;

    let mut display: sh1106::mode::GraphicsMode<_> = sh1106::builder::Builder::new()
        .with_size(sh1106::displaysize::DisplaySize::Display128x64)
        .connect_i2c(i2c)
        .into();

    let numbers = vec![
        icons::FONT.sub_image(&Rectangle::new(Point::new(0, 0), Size::new(34, 64))),
        icons::FONT.sub_image(&Rectangle::new(Point::new(34, 0), Size::new(22, 64))),
        icons::FONT.sub_image(&Rectangle::new(Point::new(57, 0), Size::new(34, 64))),
        icons::FONT.sub_image(&Rectangle::new(Point::new(91, 0), Size::new(33, 64))),
        icons::FONT.sub_image(&Rectangle::new(Point::new(124, 0), Size::new(39, 64))),
        icons::FONT.sub_image(&Rectangle::new(Point::new(163, 0), Size::new(35, 64))),
        icons::FONT.sub_image(&Rectangle::new(Point::new(198, 0), Size::new(33, 64))),
        icons::FONT.sub_image(&Rectangle::new(Point::new(231, 0), Size::new(33, 64))),
        icons::FONT.sub_image(&Rectangle::new(Point::new(264, 0), Size::new(34, 64))),
        icons::FONT.sub_image(&Rectangle::new(Point::new(298, 0), Size::new(36, 64))),
        icons::FONT.sub_image(&Rectangle::new(Point::new(334, 0), Size::new(22, 64))),
    ];

    display
        .init()
        .map_err(|err| anyhow!("failed to init display: {err:?}"))?;

    display.clear();

    Image::new(&icons::DISCONNECTED, Point::new(47, 16)).draw(&mut display)?;

    display.flush().unwrap();

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
                Event::Update(temperature) => {
                    draw_number(temperature, &numbers, &mut display);
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
