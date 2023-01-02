use ntpnet_lib::{net::Net, reactor::reactor, Token};
use plotmux::plotmux::{ClientMode, PlotMux};

use ntpnets::mp4_reader::MP4Reader;

fn main() {
    let mut plotmux = PlotMux::make();
    let n = Net::make()
        .set_start_tokens("image_enable", vec![Token::new(())])
        .add_transition("mp4_reader",
            MP4Reader::maker(
                plotmux.add_plot_sink("mp4_reader"),
                "./target/data/project_video.mp4".into()
            )
        )
        .place_to_transition("image_enable", "_e", "mp4_reader")
        .transition_to_place("mp4_reader", "image", "image")
    ;
    let png = n.png();
    let r = reactor(n, &mut plotmux);
    let _pm = plotmux.make_ready(Some(&png), ClientMode::Local());
    r.run(&None);
}














