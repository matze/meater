use embedded_graphics::pixelcolor::BinaryColor;

pub const DISCONNECTED: tinybmp::Bmp<BinaryColor> =
    match tinybmp::Bmp::from_slice(include_bytes!("assets/disconnected.bmp")) {
        Ok(image) => image,
        Err(_) => panic!("failed to load image"),
    };

pub const CONNECTING: tinybmp::Bmp<BinaryColor> =
    match tinybmp::Bmp::from_slice(include_bytes!("assets/connecting.bmp")) {
        Ok(image) => image,
        Err(_) => panic!("failed to load image"),
    };

pub const FONT: tinybmp::Bmp<BinaryColor> =
    match tinybmp::Bmp::from_slice(include_bytes!("assets/font.bmp")) {
        Ok(image) => image,
        Err(_) => panic!("failed to load image"),
    };
