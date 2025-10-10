pub trait CloudProvider {
    fn name(&self) -> &str;

    fn get_services(&self) -> Vec<Box<dyn CloudService>>;
}

pub trait CloudService {
    fn service_name(&self) -> &str;

    fn list_resources(&self) -> Vec<Box<dyn CloudResource>>;
}

pub trait CloudResource {}
