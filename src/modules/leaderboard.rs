//! Leaderboard computation, mirroring the site's own client-side logic
//! (`/js/leaderboard.js`): first user blood +20 pts, first root blood +30,
//! VM submission +25, writeup published +15. Everything derives from the two
//! public JSON files.

use crate::modules::machines::Machine;

#[derive(Debug, Clone, Default, PartialEq)]
pub struct LeaderboardEntry {
    pub user: String,
    pub first_user: u64,
    pub first_root: u64,
    pub vms: u64,
    pub writeups: u64,
    pub points: u64,
}

const PTS_FIRST_USER: u64 = 20;
const PTS_FIRST_ROOT: u64 = 30;
const PTS_WRITEUP: u64 = 15;

/// Accounts hidden from the site's leaderboard (`excluded` in
/// /js/leaderboard.js, snapshot 2026-09-07). Skipped in every field the way
/// the site's `getRow` does: excluded accounts earn no rank, points or stats.
const EXCLUDED_USERS: &[&str] = &["d4t4s3c"];

/// Accumulates rows with the site's `getRow` semantics: empty names and the
/// "--"/"-" placeholders are ignored, as are excluded accounts.
struct ScoreBuilder {
    scores: std::collections::HashMap<String, LeaderboardEntry>,
    excluded: std::collections::HashSet<String>,
}

impl ScoreBuilder {
    fn new(excluded: &[&str]) -> Self {
        Self {
            scores: std::collections::HashMap::new(),
            excluded: excluded.iter().map(|s| s.to_lowercase()).collect(),
        }
    }

    fn entry(&mut self, user: &str) -> Option<&mut LeaderboardEntry> {
        let key = user.trim().to_lowercase();
        if key.is_empty() || key == "--" || key == "-" {
            return None;
        }
        if self.excluded.contains(&key) {
            return None;
        }
        Some(self.scores.entry(key).or_insert_with(|| LeaderboardEntry {
            user: user.trim().to_string(),
            ..Default::default()
        }))
    }

    fn build(self) -> Vec<LeaderboardEntry> {
        let mut list: Vec<LeaderboardEntry> = self.scores.into_values().collect();
        list.sort_by(|a, b| b.points.cmp(&a.points).then(a.user.cmp(&b.user)));
        list
    }
}

/// Builds the full leaderboard from the public data, sorted by points
/// (descending), matching the site's default view.
pub fn compute(
    machines: &[Machine],
    writeups: &[(String, crate::modules::writeups::WriteupEntry)],
) -> Vec<LeaderboardEntry> {
    let mut builder = ScoreBuilder::new(EXCLUDED_USERS);
    for machine in machines {
        if let Some(e) = builder.entry(&machine.creator) {
            e.vms += 1;
            e.points += 25;
        }
        if let Some(e) = builder.entry(&machine.first_user) {
            e.first_user += 1;
            e.points += PTS_FIRST_USER;
        }
        if let Some(e) = builder.entry(&machine.first_root) {
            e.first_root += 1;
            e.points += PTS_FIRST_ROOT;
        }
    }
    for (_, writeup) in writeups {
        if let Some(e) = builder.entry(&writeup.author) {
            e.writeups += 1;
            e.points += PTS_WRITEUP;
        }
    }
    builder.build()
}

/// Zero-based position of `username` in the leaderboard, if present.
pub fn position_of(list: &[LeaderboardEntry], username: &str) -> Option<usize> {
    list.iter()
        .position(|e| e.user.eq_ignore_ascii_case(username.trim()))
        .map(|i| i + 1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::machines::Machine;
    use crate::modules::writeups::WriteupEntry;

    fn machine(first_user: &str, first_root: &str) -> Machine {
        Machine {
            first_user: first_user.into(),
            first_root: first_root.into(),
            ..Default::default()
        }
    }

    fn writeup(author: &str) -> (String, WriteupEntry) {
        (
            "x".into(),
            WriteupEntry {
                author: author.into(),
                ..Default::default()
            },
        )
    }

    #[test]
    fn computes_leaderboard_like_the_site() {
        let machines = vec![
            machine("alice", "bob"),
            machine("alice", ""),
            machine("", ""),
        ];
        let writeups = vec![writeup("alice"), writeup("bob"), writeup("bob")];
        let list = compute(&machines, &writeups);
        // alice: 2 user bloods (40) + 1 writeup (15) = 55
        // bob:   1 root blood (30) + 2 writeups (30) = 60
        assert_eq!(list[0].user, "bob");
        assert_eq!(list[0].points, 60);
        assert_eq!(list[0].first_root, 1);
        assert_eq!(list[1].user, "alice");
        assert_eq!(list[1].points, 55);
        assert_eq!(list[1].first_user, 2);
        assert_eq!(position_of(&list, "ALICE"), Some(2));
        assert_eq!(position_of(&list, "nobody"), None);
    }

    #[test]
    fn excluded_staff_is_absent_and_skipped() {
        // d4t4s3c creates machines and holds bloods; the site hides the
        // account everywhere, so it must earn neither rank nor points.
        let mut creator = machine("challenger", "challenger");
        creator.creator = "d4t4s3c".into();
        let machines = vec![
            creator,
            machine("d4t4s3c", "challenger"),
            machine("challenger", ""),
        ];
        let writeups = vec![writeup("d4t4s3c"), writeup("challenger")];
        let list = compute(&machines, &writeups);
        // challenger bloods: user m1 (20) + root m1 (30) + root m2 (30)
        // + user m3 (20) = 100, + 1 writeup (15) = 115
        assert_eq!(list[0].points, 115);
        assert_eq!(position_of(&list, "d4t4s3c"), None);
    }

    #[test]
    fn placeholders_are_ignored_like_on_the_site() {
        let machines = vec![
            machine("--", "-"),
            machine("", "  "),
            Machine::default(),
        ];
        let list = compute(&machines, &[]);
        assert!(
            list.is_empty(),
            "empty and placeholder names must not create rows"
        );
    }
}
