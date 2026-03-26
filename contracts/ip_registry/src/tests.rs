#[cfg(test)]
mod tests {
    use soroban_sdk::{BytesN, Env, Address};
    use soroban_sdk::testutils::{storage::Persistent, Ledger as _};
    use crate::IpRegistryClient;

    use super::{IpRegistry, DataKey, IpRecord};

    #[test]
    fn test_ttl_extension_after_ip_commit() {
        let env = Env::default();
        let contract_id = env.register_contract_wasm(None, ip_registry_wasm);
        let client = IpRegistryClient::new(&env, &contract_id);

        let owner = Address::from_str(&env, "GD726F6N6A6S6B6A6S6B6A6S6B6A6S6B6A6S6B6A6S6B6A6S6B6A6S6B6A6S6B6A6S6B");
        let commitment_hash = BytesN::from_array(&env, &[0; 32]);

        // Commit an IP
        let ip_id = client.commit_ip(&owner, &commitment_hash);

        // Verify the record exists
        let record = client.get_ip(&ip_id);
        assert_eq!(record.owner, owner);

        // Check that TTL was extended
        let ttl = env.storage().persistent().get_ttl(&DataKey::IpRecord(ip_id)).unwrap();
        assert!(ttl > 0, "TTL should be extended after write");

        // Simulate ledger progression
        env.jump(1000);

        // Record should still be accessible
        let record_after = client.get_ip(&ip_id);
        assert_eq!(record_after.owner, owner);
        assert_eq!(record_after.commitment_hash, commitment_hash);
    }

    #[test]
    fn test_ttl_extension_after_owner_ips_update() {
        let env = Env::default();
        let contract_id = env.register_contract_wasm(None, ip_registry_wasm);
        let client = IpRegistryClient::new(&env, &contract_id);

        let owner = Address::from_str(&env, "GD726F6N6A6S6B6A6S6B6A6S6B6A6S6B6A6S6B6A6S6B6A6S6B6A6S6B6A6S6B6A6S6B");
        let commitment_hash = BytesN::from_array(&env, &[0; 32]);

        // Commit first IP
        let ip_id1 = client.commit_ip(&owner, &commitment_hash);

        // Check OwnerIps TTL
        let ttl = env.storage().persistent().get_ttl(&DataKey::OwnerIps(owner.clone())).unwrap();
        assert!(ttl > 0, "OwnerIps TTL should be extended after write");

        // Commit second IP
        let commitment_hash2 = BytesN::from_array(&env, &[1; 32]);
        let ip_id2 = client.commit_ip(&owner, &commitment_hash2);

        // Verify both IPs are listed
        let ip_list = client.list_ip_by_owner(&owner);
        assert_eq!(ip_list.len(), 2);

        // Simulate ledger progression
        env.jump(1000);

        // Owner IPs should still be accessible
        let ip_list_after = client.list_ip_by_owner(&owner);
        assert_eq!(ip_list_after.len(), 2);
    }

    #[test]
    fn test_multiple_ttl_extensions() {
        let env = Env::default();
        let contract_id = env.register_contract_wasm(None, ip_registry_wasm);
        let client = IpRegistryClient::new(&env, &contract_id);

        let owner = Address::from_str(&env, "GD726F6N6A6S6B6A6S6B6A6S6B6A6S6B6A6S6B6A6S6B6A6S6B6A6S6B6A6S6B6A6S6B");
        let commitment_hash = BytesN::from_array(&env, &[0; 32]);

        // Commit IP
        let ip_id = client.commit_ip(&owner, &commitment_hash);

        let initial_ttl = env.storage().persistent().get_ttl(&DataKey::IpRecord(ip_id)).unwrap();

        // Simulate some ledger progression
        env.jump(100);

        // Access the record (this doesn't extend TTL, but verifies it's still there)
        let _record = client.get_ip(&ip_id);

        let current_ttl = env.storage().persistent().get_ttl(&DataKey::IpRecord(ip_id)).unwrap();
        assert!(current_ttl < initial_ttl, "TTL should have decreased over time");

        // The record should still be accessible
        let record = client.get_ip(&ip_id);
        assert_eq!(record.owner, owner);
    }
}
