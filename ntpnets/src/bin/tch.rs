use anyhow::Result;
use tch::{nn, nn::Module, nn::OptimizerConfig, Device};

const IMAGE_DIM: i64 = 784;
const HIDDEN_NODES: i64 = 128;
const LABELS: i64 = 10;

fn net(vs: &nn::Path) -> impl Module {
    nn::seq()
        .add(nn::linear(
            vs / "layer1",
            IMAGE_DIM,
            HIDDEN_NODES,
            Default::default(),
        ))
        .add_fn(|xs| xs.relu())
        .add(nn::linear(vs, HIDDEN_NODES, LABELS, Default::default()))
}

pub fn main() -> Result<()> {
    use plotmux::plotmux::PlotMux;
    let mut plotmux = PlotMux::make();
    let mut plotsink = plotmux.add_plot_sink("training");
    plotmux.make_ready(None);
    let data_path = std::env::current_exe()
        .expect("Getting current exe")
        .as_path()
        .parent().unwrap()
        .parent().unwrap()
        .join(std::path::Path::new("data"));
    let m = tch::vision::mnist::load_dir(&data_path)?;
    let vs = nn::VarStore::new(Device::Cpu);
    let net = net(&vs.root());
    let mut opt = nn::Adam::default().build(&vs, 1e-3)?;
    for epoch in 1..200 {
        let loss = net
            .forward(&m.train_images)
            .cross_entropy_for_logits(&m.train_labels);
        opt.backward_step(&loss);
        let test_accuracy = net
            .forward(&m.test_images)
            .accuracy_for_logits(&m.test_labels);
        plotsink.plot_series_2d("", "train loss", epoch as f64, f64::from(&loss));
        plotsink.plot_series_2d("", "test acc", epoch as f64, f64::from(&test_accuracy));
        plotsink.println(&format!(
            "epoch: {:4} train loss: {:8.5} test acc: {:5.2}%",
            epoch,
            f64::from(&loss),
            100. * f64::from(&test_accuracy),
        ));
    }
    Ok(())
}
