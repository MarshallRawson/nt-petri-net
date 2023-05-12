use utilities::{camera_reader::CameraReader, facial_recognition::FacialRecognition};

use ntpnet::{Net, reactor, Token, ReactorOptions};
use plotmux::plotmux::{PlotMux, ClientMode};

use clap::Parser;
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

    let mut plotmux = PlotMux::make();
    let n = Net::make()
        .set_start_tokens("E", vec![Token::new(())])
        .place_to_transition("E", "_enable", "camera_reader")
        .add_transition(
            "camera_reader",
            CameraReader::maker(
                args.fps,
                args.dev,
                plotmux.add_plot_sink("camera_reader"),
            ),
        )
        .add_transition(
            "image_consumer",
            FacialRecognition::maker(plotmux.add_plot_sink("image_consumer")),
        )
        .transition_to_place("camera_reader", "image", "Image")
        .place_to_transition("Image", "image", "image_consumer")
        .transition_to_place("image_consumer", "out", "E");
    let png = n.png();
    let r = reactor(n, &mut plotmux);
    let plotmux_mode = if let Some(addr) = args.remote_plotmux {
        ClientMode::Remote(addr)
    } else {
        ClientMode::Local()
    };
    let pm = plotmux.make_ready(Some(&png), plotmux_mode);
    r.run(&args.reactor_plot_options);
    drop(pm);
}
