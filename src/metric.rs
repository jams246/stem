//! Metric and band definitions shared by scanning, scoring and reporting.

/// One of the four per-function measurements.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Metric {
    /// How hard the logic is to follow.
    CognitiveComplexity,
    /// Lines of code in the function.
    Length,
    /// Values every caller must pass, in order.
    Parameters,
    /// Different names the reader must keep track of.
    Vocabulary,
}

impl Metric {
    /// All metrics in their fixed reporting order.
    pub const ALL: [Metric; 4] = [
        Metric::CognitiveComplexity,
        Metric::Length,
        Metric::Parameters,
        Metric::Vocabulary,
    ];

    /// Position of the metric in [`Metric::ALL`].
    pub fn index(self) -> usize {
        self as usize
    }

    /// Identifier used by `stem explain`.
    pub fn id(self) -> &'static str {
        match self {
            Metric::CognitiveComplexity => "cognitive-complexity",
            Metric::Length => "length",
            Metric::Parameters => "parameters",
            Metric::Vocabulary => "vocabulary",
        }
    }

    /// camelCase key used in config files and JSON.
    pub fn key(self) -> &'static str {
        match self {
            Metric::CognitiveComplexity => "cognitiveComplexity",
            Metric::Length => "length",
            Metric::Parameters => "parameters",
            Metric::Vocabulary => "vocabulary",
        }
    }

    /// Name as shown in text output.
    pub fn label(self) -> &'static str {
        match self {
            Metric::CognitiveComplexity => "cognitive complexity",
            Metric::Length => "length",
            Metric::Parameters => "parameters",
            Metric::Vocabulary => "vocabulary",
        }
    }

    /// Short description printed next to every value.
    pub fn description(self) -> &'static str {
        match self {
            Metric::CognitiveComplexity => "how hard the logic is to follow",
            Metric::Length => "lines of code in the function",
            Metric::Parameters => "values every caller must pass, in order",
            Metric::Vocabulary => "different names the reader must keep track of",
        }
    }

    /// Default thresholds `[t1, t2, t3]`.
    pub fn default_thresholds(self) -> [u32; 3] {
        match self {
            Metric::CognitiveComplexity => [5, 10, 25],
            Metric::Length => [15, 30, 60],
            Metric::Parameters => [3, 4, 6],
            Metric::Vocabulary => [10, 20, 30],
        }
    }

    /// Looks a metric up by its camelCase key.
    pub fn from_key(key: &str) -> Option<Metric> {
        Metric::ALL.into_iter().find(|m| m.key() == key)
    }

    /// Looks a metric up by its explain identifier.
    pub fn from_id(id: &str) -> Option<Metric> {
        Metric::ALL.into_iter().find(|m| m.id() == id)
    }
}

/// How a value compares with a metric's thresholds.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Band {
    /// Up to t1.
    Low,
    /// Up to t2.
    Moderate,
    /// Up to t3.
    High,
    /// Above t3.
    VeryHigh,
}

impl Band {
    /// All bands from lowest to highest.
    pub const ALL: [Band; 4] = [Band::Low, Band::Moderate, Band::High, Band::VeryHigh];

    /// Spelled-out name used in text output.
    pub fn name(self) -> &'static str {
        match self {
            Band::Low => "low",
            Band::Moderate => "moderate",
            Band::High => "high",
            Band::VeryHigh => "very high",
        }
    }

    /// camelCase key used in JSON.
    pub fn key(self) -> &'static str {
        match self {
            Band::Low => "low",
            Band::Moderate => "moderate",
            Band::High => "high",
            Band::VeryHigh => "veryHigh",
        }
    }

    /// Cost used to rank functions by how much they have to gain.
    pub fn cost(self) -> u64 {
        match self {
            Band::Low => 0,
            Band::Moderate => 1,
            Band::High => 4,
            Band::VeryHigh => 8,
        }
    }

    /// Numeric rank, 0 for low up to 3 for very high.
    pub fn rank(self) -> u32 {
        self as u32
    }
}

/// Thresholds for all four metrics.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Thresholds(pub [[u32; 3]; 4]);

impl Default for Thresholds {
    fn default() -> Self {
        Thresholds(Metric::ALL.map(Metric::default_thresholds))
    }
}

impl Thresholds {
    /// Thresholds of one metric.
    pub fn get(&self, metric: Metric) -> [u32; 3] {
        self.0[metric.index()]
    }

    /// Replaces the thresholds of one metric.
    pub fn set(&mut self, metric: Metric, values: [u32; 3]) {
        self.0[metric.index()] = values;
    }

    /// Band of `value` for `metric`.
    pub fn band(&self, metric: Metric, value: u32) -> Band {
        let [t1, t2, t3] = self.get(metric);
        if value <= t1 {
            Band::Low
        } else if value <= t2 {
            Band::Moderate
        } else if value <= t3 {
            Band::High
        } else {
            Band::VeryHigh
        }
    }

    /// The "aim for N or less" target for a band, if any.
    pub fn target(&self, metric: Metric, band: Band) -> Option<u32> {
        let [t1, t2, _] = self.get(metric);
        match band {
            Band::Low => None,
            Band::Moderate => Some(t1),
            Band::High | Band::VeryHigh => Some(t2),
        }
    }
}
