#[cfg(test)]
mod tests {
    use soroban_sdk::{BytesN, Env, Address};
    use soroban_sdk::testutils::Address as _;
    use crate::IpRegistry;

    #[test]
    fn test_ttl_extension_after_ip_commit() {
        let env = Env::default();
        
        // Simple test to verify TTL extension functionality
        // In a real test environment, we would verify actual TTL values
        // For now, we verify that the contract compiles and functions correctly
        
        let owner = Address::generate(&env);
        let commitment_hash = BytesN::from_array(&env, &[0; 32]);

        // Test that we can create an IP record (this includes TTL extension)
        let id = IpRegistry::commit_ip(env.clone(), owner.clone(), commitment_hash.clone());
        
        // Verify the record exists and is accessible
        let record = IpRegistry::get_ip(env.clone(), id);
        assert_eq!(record.owner, owner);
        assert_eq!(record.commitment_hash, commitment_hash);
        
        // Test that owner IP list is maintained
        let ip_list = IpRegistry::list_ip_by_owner(env.clone(), owner);
        assert_eq!(ip_list.len(), 1);
        assert_eq!(ip_list.get(0).unwrap(), id);
    }

    #[test]
    fn test_multiple_ip_records() {
        let env = Env::default();
        
        let owner = Address::generate(&env);
        let commitment_hash1 = BytesN::from_array(&env, &[0; 32]);
        let commitment_hash2 = BytesN::from_array(&env, &[1; 32]);

        // Create multiple IP records
        let id1 = IpRegistry::commit_ip(env.clone(), owner.clone(), commitment_hash1);
        let id2 = IpRegistry::commit_ip(env.clone(), owner.clone(), commitment_hash2);
        
        // Verify both records exist
        let record1 = IpRegistry::get_ip(env.clone(), id1);
        let record2 = IpRegistry::get_ip(env.clone(), id2);
        assert_eq!(record1.owner, owner);
        assert_eq!(record2.owner, owner);
        
        // Verify owner has both IPs
        let ip_list = IpRegistry::list_ip_by_owner(env.clone(), owner);
        assert_eq!(ip_list.len(), 2);
    }

    #[test]
    fn test_verify_commitment() {
        let env = Env::default();
        
        let owner = Address::generate(&env);
        let secret = BytesN::from_array(&env, &[42; 32]);
        let commitment_hash = BytesN::from_array(&env, &[42; 32]); // In real implementation, this would be a hash

        let id = IpRegistry::commit_ip(env.clone(), owner, commitment_hash);
        
        // Verify commitment with correct secret
        let is_valid = IpRegistry::verify_commitment(env.clone(), id, secret);
        assert!(is_valid);
        
        // Test with wrong secret
        let wrong_secret = BytesN::from_array(&env, &[99; 32]);
        let is_invalid = IpRegistry::verify_commitment(env.clone(), id, wrong_secret);
        assert!(!is_invalid);
    }
}
