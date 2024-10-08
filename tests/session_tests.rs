
#[cfg(test)]
mod tests {
    use std::time::Duration;
    use std::thread;
    use immortal_http::session::SessionManager;
    use uuid::Uuid;

    #[test]
    fn test_session_exists() {
        let sm = SessionManager::new(
            Duration::from_secs(3),  //session_duration,
            Duration::from_secs(2),  //inactive_duration,
            Duration::from_secs(1),  //prune_rate
        );

        let nil = sm.create_session();
        assert_eq!(nil, Uuid::nil());

        sm.enable();

        let id1 = sm.create_session();
        assert!(sm.write_session(id1, "testkey", "testvalue"));
        assert_eq!(sm.read_session(id1, "testkey"), Some("testvalue".to_string()));

        sm.prune();
        assert!(sm.session_exists(id1));

        thread::sleep(Duration::from_secs(1));

        sm.prune();
        assert!(sm.session_exists(id1));

        thread::sleep(Duration::from_secs(2));

        sm.prune();
        assert!(!sm.session_exists(id1));
    }

    #[test]
    fn test_session_prune_rate() {
        let sm = SessionManager::new(
            Duration::from_secs(30),  //session_duration,
            Duration::from_secs(1),   //inactive_duration,
            Duration::from_secs(2),   //prune_rate
        );
        sm.enable();

        let id1 = sm.create_session();

        for _ in 0..2 {
            assert!(sm.session_exists(id1));
            sm.prune();
            thread::sleep(Duration::from_secs(1));
        }
        sm.prune();
        assert!(!sm.session_exists(id1));
    }

    #[test]
    fn test_session_max_duration() {
        let sm = SessionManager::new(
            Duration::from_secs(3),  //session_duration,
            Duration::from_secs(10), //inactive_duration,
            Duration::from_secs(1),  //prune_rate
        );
        sm.enable();

        let id1 = sm.create_session();

        for n in 0..3 {
            assert!(sm.session_exists(id1));
            assert!(sm.write_session(id1, format!("item{n}").as_str(), n.to_string().as_str()));
            sm.prune();
            thread::sleep(Duration::from_secs(1));
        }
        // even though the session was being used, it can only exist for 3 seconds
        sm.prune();
        assert!(!sm.session_exists(id1));
    }

    #[test]
    fn test_session_inactive_duration() {
        // 2 second inactive duration
        let sm = SessionManager::new(
            Duration::from_secs(10),  //session_duration,
            Duration::from_secs(2),   //inactive_duration,
            Duration::from_secs(1),   //prune_rate
        );
        sm.enable();

        // create and write a value to a session
        let id1 = sm.create_session();
        sm.write_session(id1, "testkey", "testvalue");

        // session should exist at 0 seconds
        sm.prune();
        assert!(sm.session_exists(id1));
        thread::sleep(Duration::from_secs(1));

        // after 1 second, check if the value is still there
        sm.prune();
        assert_eq!(sm.read_session(id1, "testkey"), Some("testvalue".to_string()));

        // session should still exist after 1 second
        sm.prune();
        assert!(sm.session_exists(id1));
        thread::sleep(Duration::from_secs(1));

        // after 2 seconds, check if the is there
        sm.prune();
        assert_eq!(sm.read_session(id1, "testkey"), Some("testvalue".to_string()));

        // finally, wait for the session to become inactive
        thread::sleep(Duration::from_secs(2));

        // should no longer exist
        sm.prune();
        assert!(!sm.session_exists(id1));
    }

    #[test]
    fn test_session_actions() {
        let sm = SessionManager::new(
            Duration::from_secs(10),   //session_duration,
            Duration::from_secs(10),   //inactive_duration,
            Duration::from_secs(10),   //prune_rate
        );
        sm.enable();

        let id1 = sm.create_session();

        assert!(sm.write_session(id1, "a", "a"));
        assert_eq!(sm.read_session(id1, "a").unwrap(), "a");

        sm.clear_session(id1);
        assert_eq!(sm.read_session(id1, "a"), None);

        sm.delete_session(id1);
        assert!(!sm.session_exists(id1));
    }
}

