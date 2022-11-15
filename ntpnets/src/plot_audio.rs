use std::time::Instant;
use rustfft::{FftPlanner, num_complex::Complex};

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
    tl: Option<Instant>,
    fs: usize,
    fft_planner: FftPlanner<f64>,
}
impl PlotAudio {
    pub fn maker(p: PlotSink) -> ntpnet_lib::TransitionMaker {
        Box::new(|| {
            Box::new(PlotAudio{
                p: p,
                start: Instant::now(),
                tl: None,
                fs: 44100,
                fft_planner: FftPlanner::new(),
            })
        })
    }
    fn audio(&mut self, i: AudioInput) -> AudioOutput {
        let (t, samples) = match i {
            AudioInput::Audio(Audio { audio: (t, samples) }) => (t, samples)
        };
        if let Some(tl) = self.tl {
            self.p.plot_series_2d("", "audio sample timing",
                (t - self.start).as_secs_f64(), (t - tl).as_secs_f64()
            );
        }
        let fft = self.fft_planner.plan_fft_forward(samples.len());
        let mut samples_complex = samples.iter().map(|x| Complex{ re: *x as f64, im: 0. }).collect::<Vec<_>>();
        fft.process(&mut samples_complex);
        self.p.plot_line_2d("frequency", "log(fft(samples))",
            samples_complex[0..samples.len()/2].iter().enumerate().map(|(x, y)|
                (
                    x as f64 / samples.len() as f64 * self.fs as f64,
                    f64::log10(y.norm())
                 )
            ).collect()
        );
        self.p.plot_line_2d("audio", "samples",
            samples.iter().enumerate().map(|(x, y)| (x as f64 / self.fs as f64, *y as f64)).collect()
        );
        self.tl = Some(t);
        AudioOutput::AudioEnable(AudioEnable{ audio_enable: () })
    }
}
