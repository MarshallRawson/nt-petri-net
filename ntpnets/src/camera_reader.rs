use image::RgbImage;
use ntpnet_lib::TransitionMaker;
use plotmux::plotsink::PlotSink;
use rscam::{Camera, Config};
use std::time::Instant;

#[derive(ntpnet_macro::TransitionInputTokens)]
struct E {
    _enable: (),
}
#[derive(ntpnet_macro::TransitionOutputTokens)]
struct Image {
    image: (Instant, RgbImage),
}
#[derive(ntpnet_macro::Transition)]
#[ntpnet_transition(read: Input(E) -> Output(Image))]
pub struct CameraReader {
    camera: Camera,
    _p: PlotSink,
}
impl CameraReader {
    pub fn maker(fps: u32, mut plotsink: PlotSink) -> TransitionMaker {
        Box::new(move || {
            let mut cam = Camera::new("/dev/video0").unwrap();
            let config = Config {
                interval: (1, fps),
                format: b"RGB3",
                ..Default::default()
            };
            plotsink.println2(
                "debug",
                &format!(
                    "frame rate: {} / {} Hz",
                    config.interval.1, config.interval.0
                ),
            );
            plotsink.println2(
                "debug",
                &format!(
                    "resolution: {} x {} Hz",
                    config.resolution.0, config.resolution.1
                ),
            );
            plotsink.println2(
                "debug",
                &format!("format: {}", std::str::from_utf8(config.format).unwrap()),
            );
            plotsink.println2("debug", &format!("nbuffers: {}", config.nbuffers));
            cam.start(&config).unwrap();
            Box::new(Self {
                camera: cam,
                _p: plotsink,
            })
        })
    }
    fn read(&mut self, _: Input) -> Output {
        let frame = self.camera.capture().unwrap();
        let now = Instant::now();
        let rgb_frame =
            RgbImage::from_raw(frame.resolution.0, frame.resolution.1, frame.to_vec()).unwrap();
        Output::Image(Image {
            image: (now, rgb_frame),
        })
    }
}
