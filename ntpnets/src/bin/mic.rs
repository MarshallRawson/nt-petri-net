use ntpnet_lib::{net::Net, reactor::Reactor};
use plotmux::plotmux::{PlotMux, ClientMode};
use ntpnets::sound_reader::SoundReader;

fn main() {
    let mut plotmux = PlotMux::make();
    let n = Net::make()
        .set_start_tokens("e", vec![Box::new(())])
        .place_to_transition("e", "_e", "sound")
        .add_transition("sound", SoundReader::maker(plotmux.add_plot_sink("sound_reader")))
        .transition_to_place("sound", "samples", "e");
    let png = n.png();
    let r = Reactor::make(n, &mut plotmux);
    plotmux.make_ready(Some(&png), ClientMode::Local());
    r.run();
}
