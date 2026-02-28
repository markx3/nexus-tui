use std::f64::consts::PI;

use crate::types::TreeNode;
use crate::widgets::tree::relative_time;

// ---------------------------------------------------------------------------
// Radar blip
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct RadarBlip {
    pub session_id: String,
    pub group_name: String,
    pub x: f64,
    pub y: f64,
    pub is_active: bool,
}

// ---------------------------------------------------------------------------
// Radar state
// ---------------------------------------------------------------------------

pub struct RadarState {
    pub sweep_angle: f64,
    pub cursor_blip: Option<usize>,
    pub blips: Vec<RadarBlip>,
}

/// Sweep rotation speed: ~6 degrees/sec = one full rotation in 60s.
const SWEEP_SPEED: f64 = 2.0 * PI / 60.0;

impl RadarState {
    pub fn new() -> Self {
        Self {
            sweep_angle: 0.0,
            cursor_blip: None,
            blips: Vec::new(),
        }
    }

    /// Advance the sweep arm by the given delta time in seconds.
    pub fn advance_sweep(&mut self, delta_secs: f64) {
        self.sweep_angle = (self.sweep_angle + SWEEP_SPEED * delta_secs) % (2.0 * PI);
    }

    /// Compute blip positions from the tree data.
    pub fn compute_blips(&mut self, tree: &[TreeNode]) {
        self.blips.clear();
        self.collect_blips(tree, "Ungrouped");

        // Preserve cursor if possible
        if let Some(idx) = self.cursor_blip {
            if idx >= self.blips.len() {
                self.cursor_blip = if self.blips.is_empty() {
                    None
                } else {
                    Some(0)
                };
            }
        }
    }

    fn collect_blips(&mut self, nodes: &[TreeNode], parent_group: &str) {
        for node in nodes {
            match node {
                TreeNode::Group(g) => {
                    self.collect_blips(&g.children, &g.name);
                }
                TreeNode::Session(s) => {
                    let group = parent_group.to_string();
                    let base_angle = hash_to_angle(&group);
                    let radius = recency_to_radius(&s.last_active);

                    // Jitter each session slightly based on session_id hash
                    let jitter_angle = small_hash_jitter(&s.session_id);
                    let jitter_radius = small_hash_radius_jitter(&s.session_id);

                    let angle = base_angle + jitter_angle;
                    let r = (radius + jitter_radius).clamp(5.0, 47.0);

                    let x = r * angle.cos();
                    let y = r * angle.sin();

                    self.blips.push(RadarBlip {
                        session_id: s.session_id.clone(),
                        group_name: group,
                        x,
                        y,
                        is_active: s.is_active,
                    });
                }
            }
        }
    }

    /// Move cursor to the next blip.
    pub fn move_cursor_next(&mut self) {
        if self.blips.is_empty() {
            self.cursor_blip = None;
            return;
        }
        self.cursor_blip = Some(match self.cursor_blip {
            Some(idx) => (idx + 1) % self.blips.len(),
            None => 0,
        });
    }

    /// Move cursor to the previous blip.
    pub fn move_cursor_prev(&mut self) {
        if self.blips.is_empty() {
            self.cursor_blip = None;
            return;
        }
        self.cursor_blip = Some(match self.cursor_blip {
            Some(0) => self.blips.len() - 1,
            Some(idx) => idx - 1,
            None => self.blips.len() - 1,
        });
    }

    /// Get the session_id of the currently selected blip.
    pub fn selected_session(&self) -> Option<&str> {
        self.cursor_blip
            .and_then(|idx| self.blips.get(idx))
            .map(|b| b.session_id.as_str())
    }

    /// Sync the cursor to a specific session by ID.
    pub fn select_by_session_id(&mut self, id: &str) {
        self.cursor_blip = self
            .blips
            .iter()
            .position(|b| b.session_id == id);
    }
}

impl Default for RadarState {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Positioning helpers
// ---------------------------------------------------------------------------

/// Hash a group name to a base angle in radians [0, 2*PI).
fn hash_to_angle(name: &str) -> f64 {
    let mut h: u32 = 0;
    for b in name.bytes() {
        h = h.wrapping_mul(31).wrapping_add(b as u32);
    }
    (h % 360) as f64 * PI / 180.0
}

/// Map recency (from relative_time) to a radius fraction.
/// Today=25%, this week=50%, this month=75%, older=95% of max radius (50).
fn recency_to_radius(iso_ts: &str) -> f64 {
    let rt = relative_time(iso_ts);
    let max_r = 47.0; // leave margin from edge

    if rt == "just now" || rt.ends_with('m') || rt.ends_with('h') {
        // Today
        max_r * 0.25
    } else if rt.ends_with('d') {
        // Check if within a week
        let days: i64 = rt.trim_end_matches('d').parse().unwrap_or(30);
        if days <= 7 {
            max_r * 0.50
        } else {
            max_r * 0.75
        }
    } else if rt.ends_with("mo") {
        max_r * 0.75
    } else {
        max_r * 0.95
    }
}

/// Small angle jitter based on session ID hash.
fn small_hash_jitter(id: &str) -> f64 {
    let mut h: u32 = 0;
    for b in id.bytes() {
        h = h.wrapping_mul(37).wrapping_add(b as u32);
    }
    // Jitter +-15 degrees
    let jitter_deg = (h % 31) as f64 - 15.0;
    jitter_deg * PI / 180.0
}

/// Small radius jitter based on session ID hash.
fn small_hash_radius_jitter(id: &str) -> f64 {
    let mut h: u32 = 0;
    for b in id.bytes() {
        h = h.wrapping_mul(41).wrapping_add(b as u32);
    }
    // Jitter +-3 units
    (h % 7) as f64 - 3.0
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mock;

    #[test]
    fn test_advance_sweep() {
        let mut state = RadarState::new();
        assert_eq!(state.sweep_angle, 0.0);

        state.advance_sweep(10.0);
        let expected = SWEEP_SPEED * 10.0;
        assert!((state.sweep_angle - expected).abs() < 1e-10);
    }

    #[test]
    fn test_advance_sweep_wraps() {
        let mut state = RadarState::new();
        // Full rotation in 60 seconds
        state.advance_sweep(60.0);
        // Should wrap back close to 0
        assert!(state.sweep_angle < 0.01);
    }

    #[test]
    fn test_compute_blips() {
        let tree = mock::mock_tree();
        let mut state = RadarState::new();
        state.compute_blips(&tree);

        // 5 sessions in the mock tree
        assert_eq!(state.blips.len(), 5);
    }

    #[test]
    fn test_blip_positions_within_bounds() {
        let tree = mock::mock_tree();
        let mut state = RadarState::new();
        state.compute_blips(&tree);

        for blip in &state.blips {
            assert!(
                blip.x.abs() <= 50.0 && blip.y.abs() <= 50.0,
                "Blip at ({}, {}) out of bounds",
                blip.x,
                blip.y
            );
        }
    }

    #[test]
    fn test_cursor_navigation() {
        let tree = mock::mock_tree();
        let mut state = RadarState::new();
        state.compute_blips(&tree);

        assert_eq!(state.cursor_blip, None);

        state.move_cursor_next();
        assert_eq!(state.cursor_blip, Some(0));

        state.move_cursor_next();
        assert_eq!(state.cursor_blip, Some(1));

        // Wrap around forward
        for _ in 0..state.blips.len() {
            state.move_cursor_next();
        }
        assert_eq!(state.cursor_blip, Some(1)); // back to 1 after wrapping

        // Prev
        state.cursor_blip = Some(0);
        state.move_cursor_prev();
        assert_eq!(state.cursor_blip, Some(state.blips.len() - 1));
    }

    #[test]
    fn test_select_by_session_id() {
        let tree = mock::mock_tree();
        let mut state = RadarState::new();
        state.compute_blips(&tree);

        state.select_by_session_id("a1b2c3d4-e5f6-7890-abcd-ef1234567890");
        assert!(state.cursor_blip.is_some());
        assert_eq!(
            state.selected_session(),
            Some("a1b2c3d4-e5f6-7890-abcd-ef1234567890")
        );
    }

    #[test]
    fn test_select_by_session_id_not_found() {
        let tree = mock::mock_tree();
        let mut state = RadarState::new();
        state.compute_blips(&tree);

        state.select_by_session_id("nonexistent");
        assert_eq!(state.cursor_blip, None);
        assert_eq!(state.selected_session(), None);
    }

    #[test]
    fn test_empty_blips_cursor() {
        let mut state = RadarState::new();
        state.move_cursor_next();
        assert_eq!(state.cursor_blip, None);
        state.move_cursor_prev();
        assert_eq!(state.cursor_blip, None);
    }

    #[test]
    fn test_hash_to_angle_deterministic() {
        let a1 = hash_to_angle("nexus");
        let a2 = hash_to_angle("nexus");
        assert_eq!(a1, a2);
    }

    #[test]
    fn test_hash_to_angle_different_names() {
        let a1 = hash_to_angle("nexus");
        let a2 = hash_to_angle("website");
        assert_ne!(a1, a2);
    }

    #[test]
    fn test_active_blip_flagged() {
        let tree = mock::mock_tree();
        let mut state = RadarState::new();
        state.compute_blips(&tree);

        let active_count = state.blips.iter().filter(|b| b.is_active).count();
        assert_eq!(active_count, 2); // feat/scanner and redesign-landing
    }
}
