#[allow(dead_code)]

macro_rules! measure_exec_time {
    ($content:expr, $output:expr) => {{
        use embedded_svc::sys_time::SystemTime;
        let start = esp_idf_svc::systime::EspSystemTime {}.now();
        let result = { $content };
        let end = esp_idf_svc::systime::EspSystemTime {}.now();
        log::info!("{} execution time: {:?}", $output, end - start);
        result
    }};
}

pub(crate) use measure_exec_time;
