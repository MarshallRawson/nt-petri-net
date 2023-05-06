use clap::Parser;
use ntpnet::{MultiReactor, Net, ReactorOptions, Token};
use ntpnets::plot_audio::PlotAudio;
use ntpnets::sound_reader::SoundReader;
use plotmux::plotmux::{ClientMode, PlotMux};

#[derive(Parser)]
struct Args {
    #[command(subcommand)]
    reactor_plot_options: Option<ReactorOptions>,
}

fn main() {
    let args = Args::parse();
    let mut plotmux = PlotMux::make();
    let n = Net::make()
        .set_start_tokens("e", vec![Token::new(())])
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
    let wc = vec![n.transitions.keys().cloned().collect()];
    let r = MultiReactor::make(n, wc, &mut plotmux);
    let pm = plotmux.make_ready(Some(&r.png()), ClientMode::Local());
    r.run(&args.reactor_plot_options);
    drop(pm);
}
