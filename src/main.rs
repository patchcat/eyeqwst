use eyeqwst::Eyeqwst;
use iced::{Application, Settings};

fn main() -> Result<(), iced::Error> {
    env_logger::init();
    Eyeqwst::run(Settings::default())
}
