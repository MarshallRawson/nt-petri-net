mod camera_reader {
    use std::time::Instant;
    use image::{RgbImage};
    use plotmux::plotsink::PlotSink;
    use nokhwa::{Camera, CameraFormat, FrameFormat};
    use ntpnet_lib::TransitionMaker;
    #[derive(ntpnet_macro::TransitionInputTokens)]
    struct E {
        _enable: (),
    }
    #[derive(ntpnet_macro::TransitionOutputTokens)]
    struct Image {
        image: RgbImage,
    }
    #[derive(ntpnet_macro::Transition)]
    #[ntpnet_transition(read: Input(E) -> Output(Image))]
    pub struct CameraReader {
        camera: Camera,
        start_time: Instant,
        last_time: Option<Instant>,
        p: PlotSink,
    }
    impl CameraReader {
        pub fn maker(width: u32, height: u32, plotsink: PlotSink) -> TransitionMaker {
            Box::new(move || {
                let mut cam = Camera::new(
                    0,
                    Some(CameraFormat::new_from(
                        width,
                        height,
                        FrameFormat::MJPEG,
                        30,
                    )),
                )
                .unwrap();
                cam.open_stream().unwrap();
                Box::new(Self {
                    camera: cam,
                    start_time: Instant::now(),
                    last_time: None,
                    p: plotsink,
                })
            })
        }
        fn read(&mut self, _: Input) -> Output {
            let resolution = self.camera.resolution();
            let frame = self.camera.frame().unwrap();
            let rgb_frame = RgbImage::from_raw(resolution.width(), resolution.height(), frame.to_vec()).unwrap();
            let now = Instant::now();
            if let Some(last_time) = self.last_time {
                self.p.plot_series_2d("frame rate".into(), (now - self.start_time).as_secs_f64(), 1. / (now - last_time).as_secs_f64());
            }
            self.last_time = Some(now);
            Output::Image(Image {
                image: rgb_frame
            })
        }
    }
}

mod image_consumer {
    use image::{RgbImage, GrayImage, ImageBuffer, Luma};
    use image::buffer::ConvertBuffer;
    use rustfft::num_complex::Complex;
    use plotmux::plotsink::PlotSink;
    use ntpnet_lib::TransitionMaker;
    use fft2d::slice::{fft_2d, ifft_2d};
    #[derive(ntpnet_macro::TransitionOutputTokens)]
    struct Out {
        out: (),
    }
    #[derive(ntpnet_macro::TransitionInputTokens)]
    struct Image {
        image: RgbImage,
    }
    #[derive(ntpnet_macro::Transition)]
    #[ntpnet_transition(consume: Input(Image) -> Output(Out))]
    pub struct ImageConsumer {
        p: PlotSink,
    }
    impl ImageConsumer {
        pub fn maker(plotsink: PlotSink) -> TransitionMaker {
            Box::new(move || Box::new(Self { p: plotsink, }))
        }
        fn consume(&mut self, i: Input) -> Output {
            let image : ImageBuffer<Luma<u8>, Vec<_>>
                = match i { Input::Image(Image { image } ) => image }.convert();
            let width = image.width() as usize;
            let height = image.height() as usize;
            let mut image_buffer = image
                .pixels()
                .map(|p| Complex::new(p[0] as f64 / 255., 0.))
                .collect::<Vec<_>>()
            ;
            fft_2d(width, height, &mut image_buffer);
            ifft_2d(height, width, &mut image_buffer);

            let fft_coef = 1.0 / (width * height) as f64;
            for x in image_buffer.iter_mut() { *x *= fft_coef; }
            let image = image_buffer.iter().map(|c| (c.norm().min(1.0) * 255.0) as u8).collect::<Vec<_>>();
            let image : RgbImage = GrayImage::from_raw(width as _, height as _, image).unwrap().convert();
            self.p.plot_image(image.convert());
            Output::Out(Out { out: () })
        }
    }
}

use ntpnet_lib::{net::Net, reactor::Reactor};
use plotmux::plotmux::PlotMux;

use clap::Parser;
#[derive(Parser)]
#[command(author, version, about, long_about = None, disable_help_flag = true)]
struct Args {
   #[arg(short, long)]
   width: u32,
   #[arg(short, long)]
   height: u32,
}

fn main() {
    let args = Args::parse();

    let mut plotmux = PlotMux::make();
    let n = Net::make()
        .set_start_tokens("E", vec![Box::new(())])
        .place_to_transition("E", "_enable", "camera_reader")
        .add_transition("camera_reader",
            camera_reader::CameraReader::maker(
                args.width, args.height,
                plotmux.add_plot_sink("camera_reader"))
            )
        .add_transition("image_consumer",
            image_consumer::ImageConsumer::maker(plotmux.add_plot_sink("image_consumer")))
        .transition_to_place("camera_reader", "image", "Image")
        .place_to_transition("Image", "image", "image_consumer")
        .transition_to_place("image_consumer", "out", "E")
    ;
    let png = n.png();
    let r = Reactor::make(n, &mut plotmux);
    plotmux.make_ready(&png);
    r.run();
}
