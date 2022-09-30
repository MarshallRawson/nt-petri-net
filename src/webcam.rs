mod camera_reader {
    use image::RgbImage;
    use plotmux::plotsink::PlotSink;
    use nokhwa::{Camera, CameraFormat, FrameFormat};
    use ntpnet_lib::TransitionMaker;
    #[derive(ntpnet_macro::TransitionInputTokens)]
    pub struct E {
        pub enable: (),
    }
    #[derive(ntpnet_macro::TransitionOutputTokens)]
    pub struct Image {
        pub image: RgbImage,
    }
    #[derive(ntpnet_macro::Transition)]
    #[ntpnet_transition(read: Input(E) -> Output(Image))]
    pub struct CameraReader {
        camera: Camera,
        p: PlotSink,
    }
    impl CameraReader {
        pub fn maker(plotsink: PlotSink) -> TransitionMaker {
            Box::new(move || {
                let width = 1920_u32;
                let height = 1080_u32;
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
        pub fn read(&mut self, _: Input) -> Output {
            let frame = self.camera.frame().unwrap();
            self.p.println("got Image!");
            Output::Image(Image { RgbImage::from_vec(frame.width(), frame.height(), frame.into_vec()).unwrap() })
        }
    }
}

use std::thread;



use show_image::{create_window, ImageInfo, ImageView, WindowProxy};

#[derive(MnetPlace)]
#[mnet_place(f, RgbImage, ())]
struct ImagePlotter {
    window: WindowProxy,
    p: PlotSink,
}
impl ImagePlotter {
    fn maker(name: String, plotmux: &mut PlotMux) -> PlaceMaker {
        let plotsink = plotmux.add_plot_sink(&name);
        PlaceMaker!(Box::new(move || Box::new(Self {
            window: create_window(&name, Default::default()).unwrap(),
            p: plotsink,
        })))
    }
    fn f(&mut self, image: RgbImage) {
        self.window
            .set_image(
                "image",
                ImageView::new(
                    ImageInfo::rgb8(image.width(), image.height()),
                    image.as_raw(),
                ),
            )
            .unwrap();
        self.p.println("plot Image!");
    }
}

#[show_image::main]
fn main() {
    let mut plotmux = PlotMux::make();
    let g = graph::Maker::make()
        .set_start_tokens::<()>("Start", vec![()])
        .edge_to_place("Start", "CameraRead")
        .add_place("CameraRead", CameraReader::maker(&mut plotmux))
        .place_to_edge("CameraRead", "Image")
        .add_edge::<RgbImage>("Image")
        .edge_to_place("Image", "PlotImage")
        .add_place(
            "PlotImage",
            ImagePlotter::maker("Camera".into(), &mut plotmux),
        )
        .place_to_edge("PlotImage", "Start");
    plotmux.make_ready(&g.png());
    thread::spawn(move || plotmux.spin());
    let e = graph::Runner::from_maker(g).run();
    println!("{:#?}", e);
}
