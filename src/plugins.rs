use crate::oto::OtoEntry;
use std::cmp::Ordering;
use regex::Regex;
use std::path::Path;

#[derive(Clone, Debug)]
pub struct ValidationIssue {
    pub severity: String, // "error", "warning", "info"
    pub message: String,
    pub row: usize,
    pub alias: String,
    pub field: Option<String>,
    pub suggested_fix: Option<f64>,
}

pub fn freq_to_note(freq: f64) -> String {
    if freq <= 0.0 { return "-".to_string(); }
    let a4 = 440.0;
    let c0 = a4 * 2.0_f64.powf(-4.75);
    if freq < c0 { return "-".to_string(); }
    
    let h = (12.0 * (freq / c0).log2()).round() as i32;
    let names = ["C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B"];
    let note_idx = (h % 12).abs() as usize;
    let octave = h / 12;
    format!("{}{}", names[note_idx], octave)
}

#[derive(Clone, Debug)]
pub struct Duplicate {
    pub row1: usize,
    pub row2: usize,
    pub alias1: String,
    pub alias2: String,
    pub match_type: String, // "exact", "case", "functional", "similar"
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SortMode {
    Alpha,
    AlphaRev,
    FileName,
    Type,
    Length,
    Offset,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SortSettings {
    pub mode: SortMode,
    pub group_by_file: bool,
    pub completed_first: bool,
}

impl Default for SortSettings {
    fn default() -> Self {
        Self {
            mode: SortMode::Alpha,
            group_by_file: false,
            completed_first: false,
        }
    }
}

pub fn sort_entries(entries: &mut Vec<OtoEntry>, settings: &SortSettings) {
    entries.sort_by(|a, b| {
        // 1. Completed first
        if settings.completed_first {
            match (a.done, b.done) {
                (true, false) => return Ordering::Less,
                (false, true) => return Ordering::Greater,
                _ => {}
            }
        }

        // 2. Group by file
        if settings.group_by_file {
            let cmp = a.filename.to_lowercase().cmp(&b.filename.to_lowercase());
            if cmp != Ordering::Equal {
                return cmp;
            }
        }

        // 3. Main mode
        match settings.mode {
            SortMode::Alpha => a.alias.to_lowercase().cmp(&b.alias.to_lowercase()),
            SortMode::AlphaRev => b.alias.to_lowercase().cmp(&a.alias.to_lowercase()),
            SortMode::FileName => a.filename.to_lowercase().cmp(&b.filename.to_lowercase()),
            SortMode::Type => {
                let ta = detect_phoneme_type(&a.alias);
                let tb = detect_phoneme_type(&b.alias);
                ta.cmp(&tb).then_with(|| a.alias.to_lowercase().cmp(&b.alias.to_lowercase()))
            }
            SortMode::Length => a.alias.len().cmp(&b.alias.len()).then_with(|| a.alias.to_lowercase().cmp(&b.alias.to_lowercase())),
            SortMode::Offset => a.offset.partial_cmp(&b.offset).unwrap_or(Ordering::Equal),
        }
    });
}

fn detect_phoneme_type(alias: &str) -> i32 {
    let a = alias.to_lowercase();
    let _vowels = "aiueoあいうえおアイウエオ";
    
    // Simplistic UTAU phoneme type detection
    // VCV: "a ka", "a あ"
    let vcv_re = Regex::new(r"^[aiueoあいうえおアイウエオ]\s+.+").unwrap();
    if vcv_re.is_match(&a) { return 1; }
    
    // VC: "a k"
    let vc_re = Regex::new(r"^[aiueoあいうえおアイウエオ]\s+[^aiueoあいうえおアイウエオ\s]+$").unwrap();
    if vc_re.is_match(&a) { return 2; }

    // Start of line / Breathing etc
    if a.starts_with('-') || a.starts_with('[') { return 4; }

    // CV: "ka"
    0 // Default CV
}

pub fn check_consistency(entries: &[OtoEntry], voicebank_dir: Option<&Path>) -> Vec<ValidationIssue> {
    let mut issues = Vec::new();

    for (row, entry) in entries.iter().enumerate() {
        // 1. Missing files
        if let Some(dir) = voicebank_dir {
            let path = dir.join(&entry.filename);
            if !path.exists() {
                issues.push(ValidationIssue {
                    severity: "error".to_string(),
                    message: format!("Arquivo não encontrado: {}", entry.filename),
                    row,
                    alias: entry.alias.clone(),
                    field: Some("filename".to_string()),
                    suggested_fix: None,
                });
            }
        }

        // 2. Empty Aliases
        if entry.alias.trim().is_empty() {
            issues.push(ValidationIssue {
                severity: "warning".to_string(),
                message: "Alias vazio".to_string(),
                row,
                alias: "(vazio)".to_string(),
                field: Some("alias".to_string()),
                suggested_fix: None,
            });
        }

        // 3. Negative values
        if entry.offset < 0.0 {
            issues.push(ValidationIssue {
                severity: "error".to_string(),
                message: format!("Offset negativo: {}ms", entry.offset),
                row,
                alias: entry.alias.clone(),
                field: Some("offset".to_string()),
                suggested_fix: Some(0.0),
            });
        }
        if entry.preutter < 0.0 {
            issues.push(ValidationIssue {
                severity: "error".to_string(),
                message: format!("Preutter negativo: {}ms", entry.preutter),
                row,
                alias: entry.alias.clone(),
                field: Some("preutter".to_string()),
                suggested_fix: Some(0.0),
            });
        }
        if entry.consonant < 0.0 {
            issues.push(ValidationIssue {
                severity: "error".to_string(),
                message: format!("Consonant negativo: {}ms", entry.consonant),
                row,
                alias: entry.alias.clone(),
                field: Some("consonant".to_string()),
                suggested_fix: Some(0.0),
            });
        }

        // 4. Overlap > Preutter
        if entry.overlap > entry.preutter && entry.preutter > 0.0 {
            issues.push(ValidationIssue {
                severity: "warning".to_string(),
                message: format!("Overlap ({}ms) maior que Preutter ({}ms)", entry.overlap, entry.preutter),
                row,
                alias: entry.alias.clone(),
                field: Some("overlap".to_string()),
                suggested_fix: Some(entry.preutter * 0.5),
            });
        }

        // 5. Consonant < Preutter (Info only usually)
        if entry.consonant < entry.preutter && entry.consonant > 0.0 {
             issues.push(ValidationIssue {
                severity: "info".to_string(),
                message: format!("Consonant ({}ms) menor que Preutter ({}ms)", entry.consonant, entry.preutter),
                row,
                alias: entry.alias.clone(),
                field: Some("consonant".to_string()),
                suggested_fix: None,
            });
        }
    }

    issues
}

pub fn detect_duplicates(entries: &[OtoEntry], check_exact: bool, check_case: bool, check_functional: bool, check_similar: bool) -> Vec<Duplicate> {
    let mut duplicates = Vec::new();
    
    // Exact/Case/Functional checks
    let mut seen_exact: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    let mut seen_case: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    let mut seen_functional: std::collections::HashMap<(String, i64, i64), usize> = std::collections::HashMap::new();

    for (i, entry) in entries.iter().enumerate() {
        let alias = &entry.alias;

        // Exact
        if check_exact {
            if let Some(&first_row) = seen_exact.get(alias) {
                duplicates.push(Duplicate {
                    row1: first_row,
                    row2: i,
                    alias1: alias.clone(),
                    alias2: alias.clone(),
                    match_type: "exact".to_string(),
                });
            } else {
                seen_exact.insert(alias.clone(), i);
            }
        }

        // Case
        if check_case && !check_exact {
            let lower = alias.to_lowercase();
            if let Some(&first_row) = seen_case.get(&lower) {
                let first_alias = &entries[first_row].alias;
                if first_alias != alias {
                    duplicates.push(Duplicate {
                        row1: first_row,
                        row2: i,
                        alias1: first_alias.clone(),
                        alias2: alias.clone(),
                        match_type: "case".to_string(),
                    });
                }
            } else {
                seen_case.insert(lower, i);
            }
        }

        // Functional
        if check_functional {
            let key = (entry.filename.clone(), (entry.offset * 10.0) as i64, (entry.cutoff * 10.0) as i64);
            if let Some(&first_row) = seen_functional.get(&key) {
                let first_alias = &entries[first_row].alias;
                if first_alias != alias {
                    duplicates.push(Duplicate {
                        row1: first_row,
                        row2: i,
                        alias1: first_alias.clone(),
                        alias2: alias.clone(),
                        match_type: "functional".to_string(),
                    });
                }
            } else {
                seen_functional.insert(key, i);
            }
        }

        // Similar (Levenshtein) - O(N^2)
        if check_similar {
            for j in 0..i {
                let d = levenshtein_distance(alias, &entries[j].alias);
                if d > 0 && d <= 2 {
                    duplicates.push(Duplicate {
                        row1: j,
                        row2: i,
                        alias1: entries[j].alias.clone(),
                        alias2: alias.clone(),
                        match_type: "similar".to_string(),
                    });
                }
            }
        }
    }

    duplicates
}

fn levenshtein_distance(s1: &str, s2: &str) -> usize {
    let v1: Vec<char> = s1.chars().collect();
    let v2: Vec<char> = s2.chars().collect();
    let n = v1.len();
    let m = v2.len();
    if n == 0 { return m; }
    if m == 0 { return n; }

    let mut dp = vec![vec![0; m + 1]; n + 1];
    for i in 0..=n { dp[i][0] = i; }
    for j in 0..=m { dp[0][j] = j; }

    for i in 1..=n {
        for j in 1..=m {
            let cost = if v1[i - 1] == v2[j - 1] { 0 } else { 1 };
            dp[i][j] = (dp[i - 1][j] + 1).min(dp[i][j - 1] + 1).min(dp[i - 1][j - 1] + cost);
        }
    }
    dp[n][m]
}

pub fn analyze_pitch(samples: &[f32], sample_rate: u32, window_ms: f64) -> (Vec<f64>, Vec<f64>) {
    let mut times = Vec::new();
    let mut pitches = Vec::new();
    
    if samples.is_empty() { return (times, pitches); }
    
    let min_freq = 50.0;
    let max_freq = 1200.0;
    let min_lag = (sample_rate as f64 / max_freq) as usize;
    let max_lag = (sample_rate as f64 / min_freq) as usize;

    let window_size = ((sample_rate as f64 * window_ms / 1000.0) as usize).max(max_lag + 1);
    let hop_size = window_size / 2;
    
    let mut i = 0;
    while i + window_size < samples.len() {
        let frame = &samples[i..i + window_size];
        let mut corr = vec![0.0; max_lag + 1];
        
        // Autocorrelation
        for lag in min_lag..=max_lag {
            let mut sum = 0.0;
            for t in 0..(window_size - lag) {
                sum += (frame[t] * frame[t + lag]) as f64;
            }
            corr[lag] = sum;
        }
        
        // Find best peak in range [min_lag, max_lag]
        let mut best_lag = 0;
        let mut max_corr = -1.0;
        
        // Use a simple local maximum finder to avoid picking "half-cycles"
        for lag in min_lag..=max_lag {
            let val = corr[lag];
            if val > max_corr {
                // Confirm it's a local peak or at least significant enough
                if lag > min_lag && lag < max_lag {
                    if val > corr[lag-1] && val > corr[lag+1] {
                        max_corr = val;
                        best_lag = lag;
                    }
                } else {
                    max_corr = val;
                    best_lag = lag;
                }
            }
        }
        
        let mut energy = 0.0;
        for &s in frame { energy += (s * s) as f64; }
        
        let freq = if max_corr > 0.4 * energy && best_lag > 0 {
            // Parabolic interpolation for better precision
            let mut final_lag = best_lag as f64;
            if best_lag > 0 && best_lag < max_lag {
                let a = corr[best_lag-1];
                let b = corr[best_lag];
                let c = corr[best_lag+1];
                let delta = (a - c) / (2.0 * (a - 2.0 * b + c));
                final_lag += delta;
            }
            sample_rate as f64 / final_lag
        } else {
            0.0
        };
        
        times.push(i as f64 * 1000.0 / sample_rate as f64);
        pitches.push(freq);
        
        i += hop_size;
    }
    
    (times, pitches)
}
