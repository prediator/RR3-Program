# Security Policy

## Supported Versions

| Version | Supported          |
| ------- | ------------------ |
| 1.0.x   | :white_check_mark: |

## Reporting a Vulnerability

If you discover a security vulnerability in the RR3 Staking smart contract, please follow these steps:

1. **DO NOT** open a public issue
2. Email security details to: [SECURITY_EMAIL]
3. Include:
   - Description of the vulnerability
   - Steps to reproduce
   - Potential impact
   - Suggested fix (if available)

## Security Measures

### Smart Contract Security
- ✅ Authority checks on all admin functions
- ✅ PDA-based account management
- ✅ Overflow protection with checked arithmetic
- ✅ Distribution round tracking (prevents double-rewarding)
- ✅ Lock period validation
- ✅ Emergency unlock with fair reward distribution

### Audit Status
- **Version 1.0**: Internal review completed
- **External Audit**: Pending

### Known Limitations
- Emergency unstake forfeits completion bonus (by design)
- 5-minute minimum between distributions (devnet testing)
- Non-standard lock periods don't receive completion bonuses

## Bug Bounty Program

Details coming soon.

## Security Best Practices for Users

1. **Never share your private keys**
2. **Verify program ID before staking**: `8HVDihB8NrYpqzRNrFuiSPUC7B4rRqa8HFRJNWoxH4JK`
3. **Check transaction details before signing**
4. **Use hardware wallets for large amounts**
5. **Understand lock periods before staking**

## Verification

Verify the deployed program matches this source code:

```bash
solana-verify verify-from-repo \
  --program-id 8HVDihB8NrYpqzRNrFuiSPUC7B4rRqa8HFRJNWoxH4JK \
  --repo https://github.com/prediator/RR3-Staking \
  --commit-hash <COMMIT_HASH> \
  --library-name staker
```

## Contact

For security-related inquiries: [SECURITY_EMAIL]
