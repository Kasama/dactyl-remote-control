use std::collections::HashMap;

use config::Config;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct I3WatcherEntry {
    #[serde(default)]
    #[serde(deserialize_with = "deserialize_string_or_seq_string")]
    pub include: Vec<String>,
    #[serde(default)]
    #[serde(deserialize_with = "deserialize_string_or_seq_string")]
    pub exclude: Vec<String>,
    pub base_layer: Option<u8>,
    pub to_layer: Option<u8>,
}

pub fn deserialize_string_or_seq_string<'de, T, D>(deserializer: D) -> Result<Vec<T>, D::Error>
where
    T: ::serde::Deserialize<'de>,
    D: ::serde::Deserializer<'de>,
{
    struct Visitor<T>(::std::marker::PhantomData<T>);

    impl<'de, T> ::serde::de::Visitor<'de> for Visitor<T>
    where
        T: ::serde::Deserialize<'de>,
    {
        type Value = Vec<T>;

        fn expecting(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
            write!(f, "a string or sequence of strings")
        }

        fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
        where
            E: ::serde::de::Error,
        {
            serde::Deserialize::deserialize(serde::de::value::StringDeserializer::new(
                v.to_string(),
            ))
            .map(|s| vec![s])
        }

        fn visit_seq<A>(self, visitor: A) -> Result<Self::Value, A::Error>
        where
            A: serde::de::SeqAccess<'de>,
        {
            serde::Deserialize::deserialize(serde::de::value::SeqAccessDeserializer::new(visitor))
        }

        fn visit_unit<E>(self) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            Ok(vec![])
        }

        fn visit_none<E>(self) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            Ok(vec![])
        }
    }

    deserializer.deserialize_any(Visitor(::std::marker::PhantomData))
}

#[derive(Debug)]
pub struct I3WatcherConfig {
    pub entries: Vec<I3WatcherEntry>,
}

#[derive(Debug, Serialize, Deserialize)]
struct I3WatcherGlobalConfig {
    exclude: Option<Vec<String>>,
    include: Option<Vec<String>>,
    base_layer: Option<u8>,
    to_layer: Option<u8>,
}

impl I3WatcherGlobalConfig {
    fn apply_defaults(&self, mut other: I3WatcherEntry) -> I3WatcherEntry {
        if let Some(ref include) = self.include {
            if other.include.is_empty() {
                other.include = include.clone();
            }
        }
        if let Some(ref exclude) = self.exclude {
            if other.exclude.is_empty() {
                other.exclude = exclude.clone();
            }
        }
        other.base_layer = other.base_layer.or(self.base_layer);
        other.to_layer = other.to_layer.or(self.to_layer);

        other
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct I3WatcherConfigFileStructure {
    global: I3WatcherGlobalConfig,
    entries: HashMap<String, I3WatcherEntry>,
}

impl I3WatcherConfig {
    pub fn load_config(config_file: &str) -> Result<Self, anyhow::Error> {
        let config = Config::builder()
            .add_source(config::File::with_name(config_file))
            .add_source(config::Environment::with_prefix("DACTYL"))
            .build()?;

        let I3WatcherConfigFileStructure {
            global: defaults,
            mut entries,
        } = config.try_deserialize()?;

        let entries = entries
            .drain()
            .map(|(_, v)| defaults.apply_defaults(v))
            .collect::<Vec<_>>();

        Ok(Self { entries })
    }

    pub fn matches_window(&self, window_name: &str) -> Option<&I3WatcherEntry> {
        self.entries.iter().find(|entry| {
            let matches_include = entry
                .include
                .iter()
                .any(|include| window_name.to_lowercase().contains(&include.to_lowercase()));
            let matches_exclude = entry
                .exclude
                .iter()
                .any(|exclude| window_name.to_lowercase().contains(&exclude.to_lowercase()));
            matches_include && !matches_exclude
        })
    }
}

#[cfg(test)]
mod test {
    #[test]
    fn test_matches_window() {
        let config = super::I3WatcherConfig {
            entries: vec![
                super::I3WatcherEntry {
                    include: vec!["foo".to_string()],
                    exclude: vec![],
                    base_layer: None,
                    to_layer: None,
                },
                super::I3WatcherEntry {
                    include: vec!["baz".to_string()],
                    exclude: vec!["bin".to_string()],
                    base_layer: None,
                    to_layer: None,
                },
            ],
        };

        assert!(config.matches_window("foo").is_some());
        assert!(config.matches_window("baz").is_some());
        assert!(config.matches_window("baz bin").is_none());
        assert!(config.matches_window("bin").is_none());
    }
}
