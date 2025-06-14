// Re-export test modules
pub mod integration;
pub mod stress;

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use tokio;

    // Common test timeout - shared across all test modules
    const DEFAULT_TEST_TIMEOUT: Duration = Duration::from_secs(5);

    // Longer timeout for stress tests
    const STRESS_TEST_TIMEOUT: Duration = Duration::from_secs(60);

    // Helper macro to run async tests with timeout
    macro_rules! async_test {
        ($test_fn:expr) => {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                tokio::time::timeout(DEFAULT_TEST_TIMEOUT, $test_fn)
                    .await
                    .unwrap()
            })
        };
        ($test_fn:expr, $timeout:expr) => {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async { tokio::time::timeout($timeout, $test_fn).await.unwrap() })
        };
    }

    #[test]
    fn test_module_initialization() {
        // This test ensures our test infrastructure is working correctly
        async_test!(async {
            // If this executes successfully, our test environment is properly set up
            assert!(true);
        });
    }

    #[test]
    fn test_stress_test_timeout() {
        // Verify stress test timeout is appropriately long
        assert!(STRESS_TEST_TIMEOUT > DEFAULT_TEST_TIMEOUT);
        assert!(STRESS_TEST_TIMEOUT >= Duration::from_secs(60));
    }
}
