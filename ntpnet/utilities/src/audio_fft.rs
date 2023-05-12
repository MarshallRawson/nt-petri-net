use rustfft::{num_complex::Complex, FftPlanner};
use std::time::Instant;

use ntpnet;
use plotmux::plotsink::PlotSink;

#[derive(ntpnet::TransitionInputTokensMacro)]
struct Audio {
    audio: (Instant, Vec<i16>),
    _enable: (),
}
#[derive(ntpnet::TransitionOutputTokensMacro)]
struct FFT {
    fft: (Instant, Vec<i16>, Vec<(f64, f64)>),
    done: (),
}
#[derive(ntpnet::Transition)]
#[ntpnet_transition(audio: AudioInput(Audio) -> AudioOutput(FFT))]
pub struct AudioFFT {
    p: PlotSink,
    start: Instant,
    tl: Option<Instant>,
    fs: usize,
    fft_planner: FftPlanner<f64>,
}
impl AudioFFT {
    pub fn maker(p: PlotSink) -> ntpnet::TransitionMaker {
        Box::new(|| {
            Box::new(AudioFFT {
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
            AudioInput::Audio(Audio {
                audio: (t, samples), ..
            }) => (t, samples),
        };
        //self.p.println(&format!("samples.len() {}", samples.len()));
        if let Some(tl) = self.tl {
            self.p.plot_series_2d(
                "",
                "audio sample timing",
                (t - self.start).as_secs_f64(),
                (t - tl).as_secs_f64(),
            );
        }
        let fft = self.fft_planner.plan_fft_forward(samples.len());
        let mut samples_complex = samples
            .iter()
            .map(|x| Complex {
                re: *x as f64,
                im: 0.,
            })
            .collect::<Vec<_>>();
        fft.process(&mut samples_complex);
        let fft_norm : Vec<_> = samples_complex[0..samples.len() / 2]
                .iter()
                .enumerate()
                .map(|(x, y)| {
                    (
                        x as f64 / samples.len() as f64 * self.fs as f64,
                        y.norm(),
                    )
                })
                .collect();
        let fft_norm_log10 : Vec<_> = fft_norm.iter().map(|(x, y)| (*x, f64::log10(*y))).collect();
        self.p.plot_line_2d(
            "frequency",
            "log(norm(fft(samples)))",
            fft_norm_log10,
        );
        self.tl = Some(t);
        AudioOutput::FFT(FFT { fft: (t, samples, fft_norm), done: () })
    }
}
