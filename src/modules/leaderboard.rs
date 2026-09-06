//! Leaderboard computation, mirroring the site's own client-side logic
//! (`/js/leaderboard.js`): first user blood +20 pts, first root blood +30,
//! writeup published +15. Everything derives from the two public JSON files.

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

/// Builds the full leaderboard from the public data, sorted by points
/// (descending), matching the site's default view.
pub fn compute(
    machines: &[Machine],
    writeups: &[(String, crate::modules::writeups::WriteupEntry)],
) -> Vec<LeaderboardEntry> {
    let mut scores: std::collections::HashMap<String, LeaderboardEntry> =
        std::collections::HashMap::new();


    for machine in machines {
        if !machine.creator.trim().is_empty() {
            let e = entry(&mut scores, &machine.creator);
            e.vms += 1;
            e.points += 25;
        }
        if !machine.first_user.trim().is_empty() {
            let e = entry(&mut scores, &machine.first_user);
            e.first_user += 1;
            e.points += PTS_FIRST_USER;
        }
        if !machine.first_root.trim().is_empty() {
            let e = entry(&mut scores, &machine.first_root);
            e.first_root += 1;
            e.points += PTS_FIRST_ROOT;
        }
    }
    for (_, writeup) in writeups {
        if !writeup.author.trim().is_empty() {
            let e = entry(&mut scores, &writeup.author);
            e.writeups += 1;
            e.points += PTS_WRITEUP;
        }
    }

    let mut list: Vec<LeaderboardEntry> = scores.into_values().collect();
    list.sort_by(|a, b| b.points.cmp(&a.points).then(a.user.cmp(&b.user)));
    list
}

/// Zero-based position of `username` in the leaderboard, if present.
pub fn position_of(list: &[LeaderboardEntry], username: &str) -> Option<usize> {
    list.iter()
        .position(|e| e.user.eq_ignore_ascii_case(username.trim()))
        .map(|i| i + 1)
}

fn entry<'a>(
    scores: &'a mut std::collections::HashMap<String, LeaderboardEntry>,
    user: &str,
) -> &'a mut LeaderboardEntry {
    let key = user.trim().to_lowercase();
    scores.entry(key).or_insert_with(|| LeaderboardEntry {
        user: user.trim().to_string(),
        ..Default::default()
    })
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
}
