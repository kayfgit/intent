//! The shape of a layout file on disk, before validation.

use std::collections::BTreeMap;

use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct LayoutSpec {
    pub layout: MetaSpec,
    #[serde(default)]
    pub thumbs: Option<ThumbsSpec>,
    #[serde(default)]
    pub spelling: Option<SpellingSpec>,
    #[serde(default, rename = "family")]
    pub families: Vec<FamilySpec>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct MetaSpec {
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub description: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ThumbsSpec {
    #[serde(default)]
    pub left: Option<String>,
    #[serde(default)]
    pub right: Option<String>,
    #[serde(default)]
    pub both: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct SpellingSpec {
    pub keys: String,
    #[serde(default)]
    pub shift: BTreeMap<String, String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct FamilySpec {
    pub id: String,
    #[serde(default)]
    pub name: Option<String>,
    pub plane: String,
    #[serde(default)]
    pub kind: KindSpec,
    pub left: String,
    #[serde(default)]
    pub members: Vec<MemberSpec>,
}

#[derive(Debug, Clone, Copy, Default, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(super) enum KindSpec {
    #[default]
    Concept,
    Text,
    Command,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub(super) enum MemberSpec {
    Simple(String),
    Detailed(DetailedMemberSpec),
}

impl MemberSpec {
    /// An empty string leaves the slot free.
    pub fn is_blank(&self) -> bool {
        matches!(self, MemberSpec::Simple(text) if text.is_empty())
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct DetailedMemberSpec {
    pub token: String,
    #[serde(default)]
    pub gloss: Option<String>,
}
