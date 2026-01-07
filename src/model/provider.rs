use std::fmt;

/// Cloud provider types supported by lazycloud.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Provider {
    /// Amazon Web Services
    Aws,
    /// Microsoft Azure
    Azure,
    /// Google Cloud Platform
    Gcp,
}

impl Provider {
    /// Human-readable display name for the provider.
    pub fn display_name(&self) -> &'static str {
        match self {
            Provider::Aws => "AWS",
            Provider::Azure => "Azure",
            Provider::Gcp => "GCP",
        }
    }

    /// Short lowercase identifier for the provider.
    pub fn id(&self) -> &'static str {
        match self {
            Provider::Aws => "aws",
            Provider::Azure => "azure",
            Provider::Gcp => "gcp",
        }
    }
}

impl fmt::Display for Provider {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.id())
    }
}
