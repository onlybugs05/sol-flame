use std::path::Path;
use serde::{Deserialize, Serialize};
use sol_flame_core::ProfileSummary;
use crate::{FlameRenderError, Result};

/// Hotspot entry in `cu-profile.json` as specified in `ARCHITECTURE.md`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CuHotspot {
    pub symbol: String,
    pub cu: u64,
    pub percent: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
}

/// JSON profile data format exported for CI/CD regression tracking.
/// Matches ARCHITECTURE.md spec:
/// ```json
/// {
///   "instruction": "deposit",
///   "total_cu": 48190,
///   "peak_heap_bytes": 4300,
///   "hotspots": [
///     {
///       "symbol": "anchor_lang::AccountDeserialize::try_deserialize",
///       "cu": 28540,
///       "percent": 59.23,
///       "source": "anchor-lang/src/lib.rs:18"
///     }
///   ]
/// }
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CuProfile {
    pub instruction: String,
    pub total_cu: u64,
    pub peak_heap_bytes: u64,
    pub hotspots: Vec<CuHotspot>,
}

impl From<&ProfileSummary> for CuProfile {
    fn from(summary: &ProfileSummary) -> Self {
        Self {
            instruction: summary.instruction.clone(),
            total_cu: summary.total_cu,
            peak_heap_bytes: summary.peak_heap_bytes,
            hotspots: summary
                .hotspots
                .iter()
                .map(|h| CuHotspot {
                    symbol: h.symbol.clone(),
                    cu: h.cu,
                    percent: h.percent,
                    source: h.source.clone(),
                })
                .collect(),
        }
    }
}

impl CuProfile {
    pub fn from_json_str(json: &str) -> Result<Self> {
        serde_json::from_str(json).map_err(FlameRenderError::Json)
    }

    pub fn to_json_string(&self) -> Result<String> {
        serde_json::to_string_pretty(self).map_err(FlameRenderError::Json)
    }

    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = std::fs::read_to_string(path)?;
        Self::from_json_str(&content)
    }

    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let content = self.to_json_string()?;
        if let Some(parent) = path.as_ref().parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, content)?;
        Ok(())
    }
}

/// Report detailing CI regression comparison between base and current commit.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CiRegressionReport {
    pub instruction: String,
    pub base_cu: u64,
    pub current_cu: u64,
    pub diff_cu: i64,
    pub percent_change: f64,
    pub max_allowed_percent: f64,
    pub passed: bool,
    pub reason: String,
}

impl CiRegressionReport {
    pub fn compare(base: &CuProfile, current: &CuProfile, max_allowed_percent: f64) -> Self {
        let diff_cu = current.total_cu as i64 - base.total_cu as i64;
        let percent_change = if base.total_cu > 0 {
            (diff_cu as f64 / base.total_cu as f64) * 100.0
        } else {
            0.0
        };

        let passed = percent_change <= max_allowed_percent;
        let reason = if passed {
            if percent_change < 0.0 {
                format!(
                    "Compute Units decreased by {:.2}% ({} CUs saved). Performance improved!",
                    percent_change.abs(),
                    diff_cu.abs()
                )
            } else {
                format!(
                    "Compute Units changed by +{:.2}%, within allowed +{:.2}% threshold.",
                    percent_change, max_allowed_percent
                )
            }
        } else {
            format!(
                "REGRESSION: Compute Units increased by +{:.2}% ({} CU increase), exceeding +{:.2}% limit!",
                percent_change, diff_cu, max_allowed_percent
            )
        };

        Self {
            instruction: current.instruction.clone(),
            base_cu: base.total_cu,
            current_cu: current.total_cu,
            diff_cu,
            percent_change: (percent_change * 100.0).round() / 100.0,
            max_allowed_percent,
            passed,
            reason,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_profile_json_roundtrip() {
        let profile = CuProfile {
            instruction: "deposit".to_string(),
            total_cu: 48190,
            peak_heap_bytes: 4300,
            hotspots: vec![CuHotspot {
                symbol: "anchor_lang::AccountDeserialize::try_deserialize".to_string(),
                cu: 28540,
                percent: 59.23,
                source: Some("anchor-lang/src/lib.rs:18".to_string()),
            }],
        };

        let json = profile.to_json_string().expect("json serialize");
        assert!(json.contains("\"instruction\": \"deposit\""));
        assert!(json.contains("28540"));

        let loaded = CuProfile::from_json_str(&json).expect("json deserialize");
        assert_eq!(profile, loaded);
    }

    #[test]
    fn test_ci_regression_comparison() {
        let base = CuProfile {
            instruction: "deposit".to_string(),
            total_cu: 50000,
            peak_heap_bytes: 4000,
            hotspots: vec![],
        };

        let regressed = CuProfile {
            instruction: "deposit".to_string(),
            total_cu: 55000, // +10%
            peak_heap_bytes: 4000,
            hotspots: vec![],
        };

        let report = CiRegressionReport::compare(&base, &regressed, 5.0);
        assert!(!report.passed);
        assert_eq!(report.percent_change, 10.0);
        assert!(report.reason.contains("REGRESSION"));

        let improved = CuProfile {
            instruction: "deposit".to_string(),
            total_cu: 45000, // -10%
            peak_heap_bytes: 4000,
            hotspots: vec![],
        };
        let rep2 = CiRegressionReport::compare(&base, &improved, 5.0);
        assert!(rep2.passed);
        assert!(rep2.reason.contains("Performance improved"));
    }
}
