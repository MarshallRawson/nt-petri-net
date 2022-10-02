use fft2d::slice::{fft_2d, ifft_2d};
use image::buffer::ConvertBuffer;
use image::{GrayImage, RgbImage};
use ntpnet_lib::TransitionMaker;
use plotmux::plotsink::PlotSink;
use rustfft::num_complex::Complex;
#[derive(ntpnet_macro::TransitionOutputTokens)]
struct Out {
    out: (),
}
#[derive(ntpnet_macro::TransitionInputTokens)]
struct Image {
    image: RgbImage,
}
#[derive(ntpnet_macro::Transition)]
#[ntpnet_transition(consume: Input(Image) -> Output(Out))]
pub struct ImageConsumer {
    p: PlotSink,
}
impl ImageConsumer {
    pub fn maker(plotsink: PlotSink) -> TransitionMaker {
        Box::new(move || Box::new(Self { p: plotsink }))
    }
    fn consume(&mut self, i: Input) -> Output {
        let image: GrayImage = match i {
            Input::Image(Image { image }) => image,
        }
        .convert();
        let width = image.width() as usize;
        let height = image.height() as usize;
        let mut image_buffer = image
            .pixels()
            .map(|p| Complex::new(p[0] as f64 / 255., 0.))
            .collect::<Vec<_>>();
        fft_2d(width, height, &mut image_buffer);
        ifft_2d(height, width, &mut image_buffer);

        let fft_coef = 1.0 / (width * height) as f64;
        for x in image_buffer.iter_mut() {
            *x *= fft_coef;
        }
        let image = image_buffer
            .iter()
            .map(|c| (c.norm().min(1.0) * 255.0) as u8)
            .collect::<Vec<_>>();
        let image: RgbImage = GrayImage::from_raw(width as _, height as _, image)
            .unwrap()
            .convert();
        self.p.plot_image(image.convert());
        Output::Out(Out { out: () })
    }
}
