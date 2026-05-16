#![allow(clippy::missing_const_for_fn, clippy::missing_errors_doc)]

use std::ffi::CString;
use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::error::{SpeechError, SpeechFrameworkError, SpeechFrameworkErrorCode};
use crate::ffi;
use crate::private::{
    cstring_from_path, cstring_from_str, error_from_status, json_cstring, take_string,
};

/// Location of a compiled custom language model and optional vocabulary.
#[derive(Debug, Clone, PartialEq)]
pub struct LanguageModelConfiguration {
    language_model: PathBuf,
    vocabulary: Option<PathBuf>,
    weight: Option<f64>,
}

impl LanguageModelConfiguration {
    #[must_use]
    pub fn new(language_model: impl AsRef<Path>) -> Self {
        Self {
            language_model: language_model.as_ref().to_path_buf(),
            vocabulary: None,
            weight: None,
        }
    }

    #[must_use]
    pub fn with_vocabulary(mut self, vocabulary: impl AsRef<Path>) -> Self {
        self.vocabulary = Some(vocabulary.as_ref().to_path_buf());
        self
    }

    #[must_use]
    pub fn with_weight(mut self, weight: f64) -> Self {
        self.weight = Some(weight);
        self
    }

    #[must_use]
    pub fn language_model(&self) -> &Path {
        &self.language_model
    }

    #[must_use]
    pub fn vocabulary(&self) -> Option<&Path> {
        self.vocabulary.as_deref()
    }

    #[must_use]
    pub const fn weight(&self) -> Option<f64> {
        self.weight
    }

    pub(crate) fn to_payload(&self) -> Result<LanguageModelConfigurationPayload, SpeechError> {
        Ok(LanguageModelConfigurationPayload {
            language_model_path: self
                .language_model
                .to_str()
                .ok_or_else(|| {
                    SpeechError::InvalidArgument("language model path is not valid UTF-8".into())
                })?
                .to_owned(),
            vocabulary_path: self
                .vocabulary
                .as_ref()
                .map(|path| {
                    path.to_str().map(str::to_owned).ok_or_else(|| {
                        SpeechError::InvalidArgument("vocabulary path is not valid UTF-8".into())
                    })
                })
                .transpose()?,
            weight: self.weight,
        })
    }

    pub(crate) fn to_json_cstring(&self) -> Result<CString, SpeechError> {
        json_cstring(&self.to_payload()?, "language model configuration")
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LanguageModelConfigurationPayload {
    pub language_model_path: String,
    pub vocabulary_path: Option<String>,
    pub weight: Option<f64>,
}

/// Utilities for building custom `SFSpeechLanguageModel` assets.
#[derive(Debug, Clone, Copy, Default)]
pub struct SpeechLanguageModel;

impl SpeechLanguageModel {
    pub fn prepare_custom_language_model(
        asset: impl AsRef<Path>,
        configuration: &LanguageModelConfiguration,
    ) -> Result<(), SpeechError> {
        Self::prepare_custom_language_model_inner(asset.as_ref(), None, configuration, false)
    }

    pub fn prepare_custom_language_model_ignoring_cache(
        asset: impl AsRef<Path>,
        configuration: &LanguageModelConfiguration,
    ) -> Result<(), SpeechError> {
        Self::prepare_custom_language_model_inner(asset.as_ref(), None, configuration, true)
    }

    #[deprecated(note = "Apple deprecated the clientIdentifier overload in macOS 26")]
    pub fn prepare_custom_language_model_with_client_identifier(
        asset: impl AsRef<Path>,
        client_identifier: &str,
        configuration: &LanguageModelConfiguration,
    ) -> Result<(), SpeechError> {
        Self::prepare_custom_language_model_inner(
            asset.as_ref(),
            Some(client_identifier),
            configuration,
            false,
        )
    }

    #[deprecated(note = "Apple deprecated the clientIdentifier overload in macOS 26")]
    pub fn prepare_custom_language_model_with_client_identifier_ignoring_cache(
        asset: impl AsRef<Path>,
        client_identifier: &str,
        configuration: &LanguageModelConfiguration,
    ) -> Result<(), SpeechError> {
        Self::prepare_custom_language_model_inner(
            asset.as_ref(),
            Some(client_identifier),
            configuration,
            true,
        )
    }

    fn prepare_custom_language_model_inner(
        asset: &Path,
        client_identifier: Option<&str>,
        configuration: &LanguageModelConfiguration,
        ignores_cache: bool,
    ) -> Result<(), SpeechError> {
        let asset_c = cstring_from_path(asset, "asset path")?;
        let config_c = configuration.to_json_cstring()?;
        let mut err_msg = std::ptr::null_mut();
        let status = if let Some(client_identifier) = client_identifier {
            let client_identifier_c = cstring_from_str(client_identifier, "client identifier")?;
            unsafe {
                ffi::sp_prepare_custom_language_model_with_client_identifier(
                    asset_c.as_ptr(),
                    client_identifier_c.as_ptr(),
                    config_c.as_ptr(),
                    ignores_cache,
                    &mut err_msg,
                )
            }
        } else {
            unsafe {
                ffi::sp_prepare_custom_language_model(
                    asset_c.as_ptr(),
                    config_c.as_ptr(),
                    ignores_cache,
                    &mut err_msg,
                )
            }
        };

        if status == ffi::status::OK {
            return Ok(());
        }

        let maybe_json = unsafe { take_string(err_msg) };
        if let Some(json) = maybe_json {
            if let Ok(error) = serde_json::from_str::<FrameworkErrorPayload>(&json) {
                let kind = SpeechFrameworkErrorCode::from_domain_code_and_message(
                    &error.domain,
                    error.code,
                    &error.localized_description,
                );
                return Err(SpeechError::Framework(SpeechFrameworkError {
                    domain: error.domain,
                    code: error.code,
                    message: error.localized_description,
                    kind,
                }));
            }
            return Err(unsafe { error_from_status(status, std::ptr::null_mut()) }
                .with_fallback_message(json));
        }

        Err(unsafe { error_from_status(status, std::ptr::null_mut()) })
    }
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct FrameworkErrorPayload {
    domain: String,
    code: i64,
    localized_description: String,
}

trait SpeechErrorExt {
    fn with_fallback_message(self, message: String) -> SpeechError;
}

impl SpeechErrorExt for SpeechError {
    fn with_fallback_message(self, message: String) -> SpeechError {
        match self {
            Self::NotAuthorized(_) => Self::NotAuthorized(message),
            Self::RecognizerUnavailable(_) => Self::RecognizerUnavailable(message),
            Self::AudioLoadFailed(_) => Self::AudioLoadFailed(message),
            Self::RecognitionFailed(_) => Self::RecognitionFailed(message),
            Self::TimedOut(_) => Self::TimedOut(message),
            Self::InvalidArgument(_) => Self::InvalidArgument(message),
            Self::Unknown { code, .. } => Self::Unknown { code, message },
            Self::Framework(error) => Self::Framework(SpeechFrameworkError { message, ..error }),
        }
    }
}
