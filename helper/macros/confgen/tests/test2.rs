#[allow(dead_code, unused_imports)]
mod test1 {
    use confgen::conf::{init_confgen, CheckFromConf, FieldCheckError};
    use confmacro::conf;
    use constructor::{Get, New, Set};
    use serde::Deserialize;

    #[derive(Get, Set, New, Debug, Deserialize, Default)]
    #[conf(lib)]
    #[set(name, version)]
    #[get(name, version, features)]
    #[new(name, version)]
    struct Cfg1 {
        name: String,
        version: String,
        features: Features,
    }

    #[test]
    fn test_default_conf1() {
        init_confgen("tests/cfg1.yaml".to_string());
        let conf = Cfg1::conf();
        println!("{:?}", conf);
        println!(
            "{:?} {:?} {:?}",
            conf.get_name(),
            conf.get_version(),
            conf.get_features()
        );
    }

    #[derive(Debug, Deserialize)]
    #[conf(path = "tests/cfg1.yaml", lib)]
    struct Cfg2 {
        name: String,
        version: String,
        features: Features,
    }

    #[test]
    fn test_target_conf2() {
        let conf = Cfg2::conf();
        println!("{:?}", conf);
    }

    #[derive(Debug, Deserialize, Default)]
    #[conf(path = "tests/cfg1.yaml", prefix = "features", lib, check)]
    struct Features {
        logging: bool,
        metrics: bool,
    }

    impl CheckFromConf for Features {
        fn _field_check(&self) -> Result<(), FieldCheckError> {
            if self.logging && self.metrics {
                let err_msg = "logging and metrics can't be true at the same time".to_string();
                println!("{}", &err_msg);
                // return Err(FieldCheckError::BizError(err_msg));
            }
            Ok(())
        }
    }

    #[test]
    fn test_prefix_conf() {
        let conf = Features::conf();
        println!("{:?}", conf);
    }
}
