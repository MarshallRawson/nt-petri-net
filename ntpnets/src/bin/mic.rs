use ntpnet_lib::{net::Net, reactor::Reactor};
use plotmux::plotmux::{PlotMux, ClientMode};
use ntpnets::sound_reader::SoundReader;

mod plot_audio {
    use std::time::Instant;
    use std::collections::VecDeque;

    use ntpnet_macro;
    use ntpnet_lib;
    use plotmux::plotsink::PlotSink;

    #[derive(ntpnet_macro::TransitionInputTokens)]
    struct Audio {
        audio: (Instant, Vec<i16>),
    }
    #[derive(ntpnet_macro::TransitionOutputTokens)]
    struct AudioEnable {
        audio_enable: (),
    }
    #[derive(ntpnet_macro::Transition)]
    #[ntpnet_transition(audio: AudioInput(Audio) -> AudioOutput(AudioEnable))]
    pub struct PlotAudio {
        p: PlotSink,
        start: Instant,
        audio: VecDeque<f32>,
        tl: Option<Instant>,
    }
    impl PlotAudio {
        pub fn maker(p: PlotSink) -> ntpnet_lib::TransitionMaker {
            Box::new(|| {
                Box::new(PlotAudio{
                    p: p,
                    start: Instant::now(),
                    audio: VecDeque::new(),
                    tl: None,
                })
            })
        }
        fn audio(&mut self, i: AudioInput) -> AudioOutput {
            let (t, samples) = match i { AudioInput::Audio(Audio { audio: (t, samples) }) => (t, samples) };
            //self.p.println(&format!("{}", samples.len() as f64 / 44100.));
            //self.process_audio(t, samples);
            AudioOutput::AudioEnable(AudioEnable{ audio_enable: () })
        }
        fn process_audio(&mut self, t: Instant, samples: Vec<i16>) {
            if let Some(tl) = self.tl {
                self.p.plot_series_2d("", "audio timing residual",
                    (t - self.start).as_secs_f64(), (t - tl).as_secs_f64()
                );
            }
            for s in samples {
                self.audio.push_back(s as f32);
                if self.audio.len() > 4410 {
                    self.audio.pop_front();
                }
            }
            self.p.plot_line_2d("audio", "", self.audio.iter().enumerate().map(|(x, y)| (x as f64, *y as f64)).collect());
            self.tl = Some(t);
        }
    }
}

fn main() {
    let mut plotmux = PlotMux::make();
    let n = Net::make()
        .set_start_tokens("e", vec![Box::new(())])
        .place_to_transition("e", "_e", "sound_reader")
        .add_transition("sound_reader", SoundReader::maker(plotmux.add_plot_sink("sound_reader")))
        .add_transition(
            "plot_audio",
            plot_audio::PlotAudio::maker(plotmux.add_plot_sink("plot_audio")),
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
