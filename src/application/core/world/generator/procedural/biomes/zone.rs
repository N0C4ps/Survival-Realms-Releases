#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum BiomeZone {
    Plains,
    Forest,
    VeryDreamcoreOneTree,
}

impl std::fmt::Display for BiomeZone {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::Plains => "Plains",
            Self::Forest => "Forest",
            Self::VeryDreamcoreOneTree => "Very Dreamcore One Tree",
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn special_zone_keeps_its_exact_design_name() {
        assert_eq!(
            BiomeZone::VeryDreamcoreOneTree.to_string(),
            "Very Dreamcore One Tree"
        );
    }
}
