use ntpnet_lib::{net::Net, reactor::Reactor};
use ntpnets::plot_audio::PlotAudio;
use ntpnets::sound_reader::SoundReader;
use plotmux::plotmux::{ClientMode, PlotMux};

fn main() {
    let mut plotmux = PlotMux::make();
    let n = Net::make()
        .set_start_tokens("e", vec![Box::new(())])
        .place_to_transition("e", "_e", "sound_reader")
        .add_transition(
            "sound_reader",
            SoundReader::maker(plotmux.add_plot_sink("sound_reader")),
        )
        .add_transition(
            "plot_audio",
            PlotAudio::maker(plotmux.add_plot_sink("plot_audio")),
        )
        .transition_to_place("plot_audio", "audio_enable", "e")
        .place_to_transition("audio", "audio", "plot_audio")
        .transition_to_place("sound_reader", "samples", "audio")
        .place_to_transition("audio", "audio", "plot_audio");
    let png = n.png();
    let r = Reactor::make(n, &mut plotmux);
    plotmux.make_ready(Some(&png), ClientMode::Local());
    r.run();
}
