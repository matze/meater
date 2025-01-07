use embedded_graphics::pixelcolor::BinaryColor;

pub const NOT_FOUND: tinybmp::Bmp<BinaryColor> =
    match tinybmp::Bmp::from_slice(include_bytes!("assets/not-found.bmp")) {
        Ok(image) => image,
        Err(_) => panic!("failed to load image"),
    };

pub const CONNECTING: tinybmp::Bmp<BinaryColor> =
    match tinybmp::Bmp::from_slice(include_bytes!("assets/connecting.bmp")) {
        Ok(image) => image,
        Err(_) => panic!("failed to load image"),
    };

pub const BATTERY_25: tinybmp::Bmp<BinaryColor> =
    match tinybmp::Bmp::from_slice(include_bytes!("assets/battery-25.bmp")) {
        Ok(image) => image,
        Err(_) => panic!("failed to load image"),
    };

pub const BATTERY_50: tinybmp::Bmp<BinaryColor> =
    match tinybmp::Bmp::from_slice(include_bytes!("assets/battery-50.bmp")) {
        Ok(image) => image,
        Err(_) => panic!("failed to load image"),
    };

pub const BATTERY_75: tinybmp::Bmp<BinaryColor> =
    match tinybmp::Bmp::from_slice(include_bytes!("assets/battery-75.bmp")) {
        Ok(image) => image,
        Err(_) => panic!("failed to load image"),
    };

pub const BATTERY_100: tinybmp::Bmp<BinaryColor> =
    match tinybmp::Bmp::from_slice(include_bytes!("assets/battery-100.bmp")) {
        Ok(image) => image,
        Err(_) => panic!("failed to load image"),
    };
