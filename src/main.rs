use eyeqwst::Eyeqwst;
use iced::{Application, Settings};

fn main() -> Result<(), iced::Error> {
    env_logger::init();
    Eyeqwst::run({
        Settings {
            fonts: vec![
                include_bytes!("../assets/Roboto-BlackItalic.ttf").into(),
                include_bytes!("../assets/Roboto-Black.ttf").into(),
                include_bytes!("../assets/Roboto-BoldItalic.ttf").into(),
                include_bytes!("../assets/Roboto-Bold.ttf").into(),
                include_bytes!("../assets/Roboto-Italic.ttf").into(),
                include_bytes!("../assets/Roboto-LightItalic.ttf").into(),
                include_bytes!("../assets/Roboto-Light.ttf").into(),
                include_bytes!("../assets/Roboto-MediumItalic.ttf").into(),
                include_bytes!("../assets/Roboto-Medium.ttf").into(),
                include_bytes!("../assets/Roboto-Regular.ttf").into(),
                include_bytes!("../assets/Roboto-ThinItalic.ttf").into(),
                include_bytes!("../assets/Roboto-Thin.ttf").into(),
            ],
            default_font: eyeqwst::DEFAULT_FONT,
            ..Settings::default()
        }
    })
}
