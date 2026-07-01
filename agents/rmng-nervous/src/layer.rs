use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

/// RMNG multi-level agent layer (ADR-017).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum AgentLayer {
    /// Core / hardware — kernel, devices, low-level ops (high privilege).
    L1,
    /// Runtime / execution — tools, MCP proxy, audit (medium-high).
    L2,
    /// Integration / domain — specialized workflows (medium).
    L3,
    /// Orchestration / swarm — planning and delegation only (low).
    L4,
}

impl AgentLayer {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::L1 => "L1",
            Self::L2 => "L2",
            Self::L3 => "L3",
            Self::L4 => "L4",
        }
    }

    pub fn privilege_label(self) -> &'static str {
        match self {
            Self::L1 => "high",
            Self::L2 => "medium-high",
            Self::L3 => "medium",
            Self::L4 => "low",
        }
    }

    /// Handoffs may only flow downward (higher layer → lower layer).
    pub fn can_delegate_to(self, target: AgentLayer) -> bool {
        (self as u8) > (target as u8)
    }

    pub fn numeric(self) -> u8 {
        match self {
            Self::L1 => 1,
            Self::L2 => 2,
            Self::L3 => 3,
            Self::L4 => 4,
        }
    }
}

impl fmt::Display for AgentLayer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl FromStr for AgentLayer {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_uppercase().as_str() {
            "L1" | "1" => Ok(Self::L1),
            "L2" | "2" => Ok(Self::L2),
            "L3" | "3" => Ok(Self::L3),
            "L4" | "4" => Ok(Self::L4),
            other => Err(format!("unknown agent layer: {other}")),
        }
    }
}

/// Trait for layer-aware agents loaded from YAML definitions.
pub trait LayerAgent {
    fn layer(&self) -> AgentLayer;
    fn can_handoff_to(&self, target: &dyn LayerAgent) -> bool;
    fn privilege(&self) -> &'static str {
        self.layer().privilege_label()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn l4_delegates_downward_only() {
        assert!(AgentLayer::L4.can_delegate_to(AgentLayer::L1));
        assert!(AgentLayer::L4.can_delegate_to(AgentLayer::L3));
        assert!(!AgentLayer::L1.can_delegate_to(AgentLayer::L4));
        assert!(!AgentLayer::L3.can_delegate_to(AgentLayer::L3));
    }
}
