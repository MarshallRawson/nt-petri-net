mod camera_reader {
    use image::RgbImage;
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
                    p: plotsink,
                })
            })
        }
        fn read(&mut self, _: Input) -> Output {
            let frame = self.camera.frame().unwrap();
            self.p.println("got Image!");
            Output::Image(Image { image: RgbImage::from_vec(frame.width(), frame.height(), frame.into_vec()).unwrap() })
        }
    }
}

mod image_consumer {
    use image::RgbImage;
    use plotmux::plotsink::PlotSink;
    use ntpnet_lib::TransitionMaker;
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
            self.p.println("got Image 2!");
            match i {
                Input::Image(Image { image } ) => {
                    self.p.plot_image(image);
                }
            };
            Output::Out(Out { out: () })
        }
    }
}

use ntpnet_lib::{net::Net, reactor::Reactor};
use plotmux::plotmux::PlotMux;
use std::thread;

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
    plotmux.make_ready(&n.png());
    thread::spawn(move || plotmux.spin());
    Reactor::make(n).run();
}
