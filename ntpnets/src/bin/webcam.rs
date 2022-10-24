use ntpnets::camera_reader::CameraReader;
use ntpnets::image_consumer::ImageConsumer;

use ntpnet_lib::{net::Net, reactor::Reactor};
use plotmux::plotmux::PlotMux;

use clap::Parser;
#[derive(Parser)]
#[command(author, version, about, long_about = None, disable_help_flag = true)]
struct Args {
    #[arg(short, long, default_value_t = 30)]
    fps: u32,
}

fn main() {
    let args = Args::parse();

    let mut plotmux = PlotMux::make();
    let n = Net::make()
        .set_start_tokens("E", vec![Box::new(())])
        .place_to_transition("E", "_enable", "camera_reader")
        .add_transition(
            "camera_reader",
            CameraReader::maker(
                args.fps,
                plotmux.add_plot_sink("camera_reader"),
            ),
        )
        .add_transition(
            "image_consumer",
            ImageConsumer::maker(plotmux.add_plot_sink("image_consumer")),
        )
        .transition_to_place("camera_reader", "image", "Image")
        .place_to_transition("Image", "image", "image_consumer")
        .transition_to_place("image_consumer", "out", "E");
    let png = n.png();
    let r = Reactor::make(n, &mut plotmux);
    plotmux.make_ready(Some(&png));
    r.run();
}
