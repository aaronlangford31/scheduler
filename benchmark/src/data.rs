use rand::StdRng;
use statrs::distribution::{Distribution, Exponential, Gamma};

pub struct F64exponentialUncertain {
    pub rate: f64,
    pub scale: f64,
}

pub struct F64gammaUncertain {
    pub shape: f64,
    pub rate: f64,
    pub scale: f64,
}

pub fn generate_task_data(
    n_tasks: usize,
    task_rate: F64exponentialUncertain,
    task_size: F64gammaUncertain,
) -> Vec<(f64, f64)> {
    let gamma_distribution = Gamma::new(task_size.shape, task_size.rate).unwrap();
    let expon_distribution = Exponential::new(task_rate.rate).unwrap();
    let mut random_number_generator = StdRng::new().unwrap();

    let mut data = Vec::with_capacity(n_tasks);
    for _ in 0..n_tasks {
        let delay =
            expon_distribution.sample::<StdRng>(&mut random_number_generator) * task_rate.scale;
        let size =
            gamma_distribution.sample::<StdRng>(&mut random_number_generator) * task_size.scale;
        data.push((delay, size));
    }

    data
}
