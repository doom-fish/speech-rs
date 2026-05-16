#![allow(
    clippy::iter_without_into_iter,
    clippy::missing_const_for_fn,
    clippy::missing_errors_doc,
    clippy::module_name_repetitions,
    clippy::needless_pass_by_value,
    clippy::use_self
)]

use std::collections::BTreeMap;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::error::SpeechError;
use crate::ffi;
use crate::private::{
    cstring_from_path, cstring_from_str, error_from_status_or_json, json_cstring,
    parse_json_ptr,
};

/// Trait mirroring Speech's `DataInsertable` protocol.
pub trait DataInsertable {
    fn append_to(&self, builder: &mut DataInsertableBuilder);
}

/// Trait mirroring Speech's `TemplateInsertable` protocol.
pub trait TemplateInsertable {
    fn append_to(&self, builder: &mut TemplateInsertableBuilder);
}

/// `SFCustomLanguageModelData.PhraseCount`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PhraseCount {
    pub phrase: String,
    pub count: usize,
}

impl PhraseCount {
    #[must_use]
    pub fn new(phrase: impl Into<String>, count: usize) -> Self {
        Self {
            phrase: phrase.into(),
            count,
        }
    }
}

impl DataInsertable for PhraseCount {
    fn append_to(&self, builder: &mut DataInsertableBuilder) {
        builder.items.push(DataInsertableItem::PhraseCount {
            phrase: self.phrase.clone(),
            count: self.count,
        });
    }
}

/// `SFCustomLanguageModelData.CustomPronunciation`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CustomPronunciation {
    pub grapheme: String,
    pub phonemes: Vec<String>,
}

impl CustomPronunciation {
    #[must_use]
    pub fn new<I, S>(grapheme: impl Into<String>, phonemes: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Self {
            grapheme: grapheme.into(),
            phonemes: phonemes.into_iter().map(Into::into).collect(),
        }
    }
}

impl DataInsertable for CustomPronunciation {
    fn append_to(&self, builder: &mut DataInsertableBuilder) {
        builder
            .items
            .push(DataInsertableItem::CustomPronunciation {
                grapheme: self.grapheme.clone(),
                phonemes: self.phonemes.clone(),
            });
    }
}

/// Builder mirroring `SFCustomLanguageModelData.DataInsertableBuilder`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DataInsertableBuilder {
    items: Vec<DataInsertableItem>,
}

impl DataInsertableBuilder {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn with<T: DataInsertable>(mut self, value: T) -> Self {
        value.append_to(&mut self);
        self
    }

    pub fn push<T: DataInsertable>(&mut self, value: T) {
        value.append_to(self);
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
}

impl DataInsertable for DataInsertableBuilder {
    fn append_to(&self, builder: &mut DataInsertableBuilder) {
        builder.items.extend(self.items.clone());
    }
}

/// `SFCustomLanguageModelData.PhraseCountGenerator`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PhraseCountGenerator {
    values: Vec<PhraseCount>,
}

impl PhraseCountGenerator {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn with_phrase_count(mut self, phrase_count: PhraseCount) -> Self {
        self.push(phrase_count);
        self
    }

    pub fn push(&mut self, phrase_count: PhraseCount) {
        self.values.push(phrase_count);
    }

    #[must_use]
    pub fn iter(&self) -> PhraseCountGeneratorIterator {
        PhraseCountGeneratorIterator {
            inner: self.values.clone().into_iter(),
        }
    }

    #[must_use]
    pub fn values(&self) -> &[PhraseCount] {
        &self.values
    }
}

impl DataInsertable for PhraseCountGenerator {
    fn append_to(&self, builder: &mut DataInsertableBuilder) {
        builder.items.push(DataInsertableItem::PhraseCountGenerator {
            values: self.values.clone(),
        });
    }
}

/// Iterator mirroring `SFCustomLanguageModelData.PhraseCountGenerator.Iterator`.
#[derive(Debug, Clone)]
pub struct PhraseCountGeneratorIterator {
    inner: std::vec::IntoIter<PhraseCount>,
}

impl Iterator for PhraseCountGeneratorIterator {
    type Item = PhraseCount;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next()
    }
}

/// `SFCustomLanguageModelData.TemplatePhraseCountGenerator.Template`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TemplatePhraseCountGeneratorTemplate {
    pub body: String,
    pub count: usize,
}

impl TemplatePhraseCountGeneratorTemplate {
    #[must_use]
    pub fn new(body: impl Into<String>, count: usize) -> Self {
        Self {
            body: body.into(),
            count,
        }
    }
}

impl TemplateInsertable for TemplatePhraseCountGeneratorTemplate {
    fn append_to(&self, builder: &mut TemplateInsertableBuilder) {
        builder.items.push(TemplateInsertableItem::Template {
            body: self.body.clone(),
            count: self.count,
        });
    }
}

/// `SFCustomLanguageModelData.CompoundTemplate`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompoundTemplate {
    items: Vec<TemplateInsertableItem>,
}

impl CompoundTemplate {
    #[must_use]
    pub fn new<T, I>(templates: I) -> Self
    where
        I: IntoIterator<Item = T>,
        T: TemplateInsertable,
    {
        let mut builder = TemplateInsertableBuilder::new();
        for template in templates {
            template.append_to(&mut builder);
        }
        Self { items: builder.items }
    }
}

impl TemplateInsertable for CompoundTemplate {
    fn append_to(&self, builder: &mut TemplateInsertableBuilder) {
        builder.items.push(TemplateInsertableItem::CompoundTemplate {
            components: self.items.clone(),
        });
    }
}

/// Builder mirroring `SFCustomLanguageModelData.TemplateInsertableBuilder`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TemplateInsertableBuilder {
    items: Vec<TemplateInsertableItem>,
}

impl TemplateInsertableBuilder {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn with<T: TemplateInsertable>(mut self, value: T) -> Self {
        value.append_to(&mut self);
        self
    }

    pub fn push<T: TemplateInsertable>(&mut self, value: T) {
        value.append_to(self);
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
}

impl TemplateInsertable for TemplateInsertableBuilder {
    fn append_to(&self, builder: &mut TemplateInsertableBuilder) {
        builder.items.extend(self.items.clone());
    }
}

/// `SFCustomLanguageModelData.TemplatePhraseCountGenerator`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TemplatePhraseCountGenerator {
    templates: Vec<TemplatePhraseCountGeneratorTemplate>,
    template_classes: BTreeMap<String, Vec<String>>,
}

impl TemplatePhraseCountGenerator {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn with_template(mut self, body: impl Into<String>, count: usize) -> Self {
        self.insert_template(body, count);
        self
    }

    pub fn insert_template(&mut self, body: impl Into<String>, count: usize) {
        self.templates
            .push(TemplatePhraseCountGeneratorTemplate::new(body, count));
    }

    pub fn define_class<I, S>(&mut self, class_name: impl Into<String>, values: I)
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.template_classes.insert(
            class_name.into(),
            values.into_iter().map(Into::into).collect(),
        );
    }

    #[must_use]
    pub fn templates(&self) -> &[TemplatePhraseCountGeneratorTemplate] {
        &self.templates
    }

    #[must_use]
    pub fn template_classes(&self) -> &BTreeMap<String, Vec<String>> {
        &self.template_classes
    }

    #[must_use]
    pub fn iter(&self) -> TemplatePhraseCountGeneratorIterator {
        TemplatePhraseCountGeneratorIterator {
            inner: expand_templates(&self.templates, &self.template_classes).into_iter(),
        }
    }
}

impl DataInsertable for TemplatePhraseCountGenerator {
    fn append_to(&self, builder: &mut DataInsertableBuilder) {
        builder
            .items
            .push(DataInsertableItem::TemplatePhraseCountGenerator {
                templates: self
                    .templates
                    .iter()
                    .map(|template| TemplateInsertableItem::Template {
                        body: template.body.clone(),
                        count: template.count,
                    })
                    .collect(),
                classes: self.template_classes.clone(),
            });
    }
}

/// Iterator mirroring `SFCustomLanguageModelData.TemplatePhraseCountGenerator.Iterator`.
#[derive(Debug, Clone)]
pub struct TemplatePhraseCountGeneratorIterator {
    inner: std::vec::IntoIter<PhraseCount>,
}

impl Iterator for TemplatePhraseCountGeneratorIterator {
    type Item = PhraseCount;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next()
    }
}

/// `SFCustomLanguageModelData.PhraseCountsFromTemplates`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PhraseCountsFromTemplates {
    classes: BTreeMap<String, Vec<String>>,
    templates: Vec<TemplateInsertableItem>,
}

impl PhraseCountsFromTemplates {
    #[must_use]
    pub fn new(classes: BTreeMap<String, Vec<String>>, builder: TemplateInsertableBuilder) -> Self {
        Self {
            classes,
            templates: builder.items,
        }
    }

    #[must_use]
    pub fn expanded_phrase_counts(&self) -> Vec<PhraseCount> {
        let templates = template_items_to_templates(&self.templates);
        expand_templates(&templates, &self.classes)
    }
}

impl DataInsertable for PhraseCountsFromTemplates {
    fn append_to(&self, builder: &mut DataInsertableBuilder) {
        builder.items.push(DataInsertableItem::PhraseCountsFromTemplates {
            classes: self.classes.clone(),
            templates: self.templates.clone(),
        });
    }
}

/// Safe Rust wrapper for `SFCustomLanguageModelData`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SFCustomLanguageModelData {
    locale_identifier: String,
    identifier: String,
    version: String,
    items: Vec<DataInsertableItem>,
}

impl SFCustomLanguageModelData {
    #[must_use]
    pub fn new(
        locale_identifier: impl Into<String>,
        identifier: impl Into<String>,
        version: impl Into<String>,
    ) -> Self {
        Self {
            locale_identifier: locale_identifier.into(),
            identifier: identifier.into(),
            version: version.into(),
            items: Vec::new(),
        }
    }

    #[must_use]
    pub fn locale_identifier(&self) -> &str {
        &self.locale_identifier
    }

    #[must_use]
    pub fn identifier(&self) -> &str {
        &self.identifier
    }

    #[must_use]
    pub fn version(&self) -> &str {
        &self.version
    }

    pub fn supported_phonemes(locale_identifier: &str) -> Result<Vec<String>, SpeechError> {
        let locale_identifier = cstring_from_str(locale_identifier, "language model locale identifier")?;
        let mut json = std::ptr::null_mut();
        let mut err_msg = std::ptr::null_mut();
        let status = unsafe {
            ffi::sp_custom_language_model_supported_phonemes_json(
                locale_identifier.as_ptr(),
                &mut json,
                &mut err_msg,
            )
        };
        if status != ffi::status::OK {
            return Err(unsafe { error_from_status_or_json(status, err_msg) });
        }
        unsafe { parse_json_ptr::<Vec<String>>(json, "supported phonemes") }
    }

    pub fn insert<T: DataInsertable>(&mut self, value: T) {
        let mut builder = DataInsertableBuilder::new();
        value.append_to(&mut builder);
        self.items.extend(builder.items);
    }

    #[must_use]
    pub fn with_insertable<T: DataInsertable>(mut self, value: T) -> Self {
        self.insert(value);
        self
    }

    pub fn export_to(&self, path: impl AsRef<Path>) -> Result<(), SpeechError> {
        let path = cstring_from_path(path.as_ref(), "custom language model export path")?;
        let json =
            json_cstring(&CustomLanguageModelDataPayload::from(self), "custom language model data")?;
        let mut err_msg = std::ptr::null_mut();
        let status = unsafe {
            ffi::sp_custom_language_model_export(json.as_ptr(), path.as_ptr(), &mut err_msg)
        };
        if status == ffi::status::OK {
            Ok(())
        } else {
            Err(unsafe { error_from_status_or_json(status, err_msg) })
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
enum DataInsertableItem {
    PhraseCount { phrase: String, count: usize },
    CustomPronunciation { grapheme: String, phonemes: Vec<String> },
    PhraseCountGenerator { values: Vec<PhraseCount> },
    TemplatePhraseCountGenerator {
        templates: Vec<TemplateInsertableItem>,
        classes: BTreeMap<String, Vec<String>>,
    },
    PhraseCountsFromTemplates {
        classes: BTreeMap<String, Vec<String>>,
        templates: Vec<TemplateInsertableItem>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
enum TemplateInsertableItem {
    Template { body: String, count: usize },
    CompoundTemplate { components: Vec<TemplateInsertableItem> },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
struct CustomLanguageModelDataPayload {
    locale_identifier: String,
    identifier: String,
    version: String,
    items: Vec<DataInsertableItem>,
}

impl From<&SFCustomLanguageModelData> for CustomLanguageModelDataPayload {
    fn from(value: &SFCustomLanguageModelData) -> Self {
        Self {
            locale_identifier: value.locale_identifier.clone(),
            identifier: value.identifier.clone(),
            version: value.version.clone(),
            items: value.items.clone(),
        }
    }
}

fn template_items_to_templates(items: &[TemplateInsertableItem]) -> Vec<TemplatePhraseCountGeneratorTemplate> {
    let mut templates = Vec::new();
    for item in items {
        match item {
            TemplateInsertableItem::Template { body, count } => {
                templates.push(TemplatePhraseCountGeneratorTemplate::new(body.clone(), *count));
            }
            TemplateInsertableItem::CompoundTemplate { components } => {
                templates.extend(template_items_to_templates(components));
            }
        }
    }
    templates
}

fn expand_templates(
    templates: &[TemplatePhraseCountGeneratorTemplate],
    classes: &BTreeMap<String, Vec<String>>,
) -> Vec<PhraseCount> {
    templates
        .iter()
        .flat_map(|template| expand_template_body(&template.body, template.count, classes))
        .collect()
}

fn expand_template_body(
    body: &str,
    count: usize,
    classes: &BTreeMap<String, Vec<String>>,
) -> Vec<PhraseCount> {
    let Some(start) = body.find('<') else {
        return vec![PhraseCount::new(body, count)];
    };
    let Some(end_offset) = body[start + 1..].find('>') else {
        return vec![PhraseCount::new(body, count)];
    };
    let end = start + 1 + end_offset;
    let class_name = &body[start + 1..end];
    let Some(values) = classes.get(class_name) else {
        return vec![PhraseCount::new(body, count)];
    };

    values
        .iter()
        .flat_map(|value| {
            let mut replaced = String::new();
            replaced.push_str(&body[..start]);
            replaced.push_str(value);
            replaced.push_str(&body[end + 1..]);
            expand_template_body(&replaced, count, classes)
        })
        .collect()
}
