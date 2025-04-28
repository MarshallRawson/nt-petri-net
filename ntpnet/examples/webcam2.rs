#[cfg(target_os = "linux")]
mod webcam2 {
    use utilities::camera_reader::CameraReader;
    use utilities::image_consumer::ImageConsumer;

    use ntpnet::{MultiReactor, Net, ReactorOptions, Token};
    use plotmux::plotmux::{ClientMode, PlotMux};

    use clap::Parser;
    use std::collections::HashSet;

    #[derive(Parser)]
    #[command(author, version, about, long_about = None)]
    struct Args {
        #[arg(short, long, default_value_t = 30)]
        fps: u32,
        #[arg(short, long, default_value_t = String::from("/dev/video0"))]
        dev: String,
        #[arg(short, long)]
        remote_plotmux: Option<String>,
        #[command(subcommand)]
        reactor_plot_options: Option<ReactorOptions>,
    }

    fn main() {
        let args = Args::parse();

        let mut plotmux = PlotMux::make(ClientMode::parse(args.remote_plotmux));
        let n = Net::make()
            .set_start_tokens("E", vec![Token::new(())])
            .place_to_transition("E", "_enable", "camera_reader")
            .add_transition(
                "camera_reader",
                CameraReader::maker(args.fps, args.dev, plotmux.add_plot_sink("camera_reader")),
            )
            .add_transition(
                "image_consumer",
                ImageConsumer::maker(plotmux.add_plot_sink("image_consumer")),
            )
            .set_start_tokens("Image", vec![])
            .transition_to_place("camera_reader", "image", "Image")
            .place_to_transition("Image", "image", "image_consumer")
            .transition_to_place("image_consumer", "out", "E");
        let multi_reactor = MultiReactor::make(
            n,
            vec![
                HashSet::from(["camera_reader".into()]),
                HashSet::from(["image_consumer".into()]),
            ],
            &mut plotmux,
        );
        let pm = plotmux.make_ready(Some(&multi_reactor.png()));
        multi_reactor.run(&args.reactor_plot_options);
        drop(pm);
    }
}

#[cfg(not(target_os = "linux"))]
fn main() {}
