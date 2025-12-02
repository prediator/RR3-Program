# RR3 Staking Smart Contract

Official Solana smart contract for RR3 token staking with professional lock periods and monthly reward distribution.

## 🔐 Verified Program

- **Program ID**: `8HVDihB8NrYpqzRNrFuiSPUC7B4rRqa8HFRJNWoxH4JK`
- **Network**: Solana Devnet
- **Framework**: Anchor v0.29+

## 📋 Features

### Lock Periods (Professional Staking)
- 3 months (7,776,000 seconds)
- 6 months (15,552,000 seconds)
- 1 year (31,536,000 seconds)
- 2 years (63,072,000 seconds)
- 3 years (94,608,000 seconds)

### Commission Structure
- **Entry Fee**: 3.33% (one-time, taken at staking)
  - Fee Wallet: 3.00%
  - Expense Wallet: 0.33%
  - Marketing Wallet: 0.03%

### Monthly Reward Multipliers
Determines the share of monthly reward pool based on lock duration:
- 3 months: 1.0000x (base)
- 6 months: 1.0100x (+1%)
- 1 year: 1.0200x (+2%)
- 2 years: 1.0300x (+3%)
- 3 years: 1.0333x (+3.33%)

### Completion Bonus Multipliers
Applied to principal when unstaking after full lock period:
- 3 months: 1.05x (5% bonus)
- 6 months: 1.12x (12% bonus)
- 1 year: 1.30x (30% bonus)
- 2 years: 1.70x (70% bonus)
- 3 years: 2.50x (150% bonus)

## 🚀 Key Functions

### User Functions
- `stake()` - Stake RR3 tokens with chosen lock period
- `claim_rr3_rewards()` - Claim monthly rewards (can be called anytime during lock)
- `unstake()` - Unstake tokens (returns principal + rewards based on lock status)

### Admin Functions
- `initialize_distribution_state()` - Initialize reward distribution system
- `record_monthly_collection()` - Record monthly RR3 allocation for distribution
- `update_total_staked()` - Update total weighted stakes
- `assign_monthly_rewards()` - Distribute rewards to individual stakers

## 📊 Reward Mechanics

### Normal Unstake (After Lock Period)
User receives:
- ✅ Principal (net staked amount after commission)
- ✅ Completion bonus (based on lock period)
- ✅ All monthly rewards earned

### Emergency Unstake (Before Lock Period)
User receives:
- ✅ Principal (net staked amount after commission)
- ✅ All monthly rewards earned (from completed cycles)
- ❌ Forfeits completion bonus (requires full lock period)

## 🔧 Building

```bash
# Install dependencies
npm install

# Build the program
anchor build

# Run tests
anchor test
```

## 🔍 Verification

This program can be verified using `solana-verify`:

```bash
solana-verify verify-from-repo \
  --program-id 8HVDihB8NrYpqzRNrFuiSPUC7B4rRqa8HFRJNWoxH4JK \
  --repo https://github.com/prediator/RR3-Staking \
  --commit-hash <COMMIT_HASH> \
  --library-name staker
```

## 📜 License

MIT License - See LICENSE file for details

## 🔒 Security

- All admin functions are protected with authority checks
- PDA (Program Derived Addresses) ensure secure account management
- Overflow protection with checked arithmetic
- Distribution round tracking prevents double-rewarding
- Lock period validation enforces professional staking only

## 🌐 Explorer Links

- [View on Solana Explorer (Devnet)](https://explorer.solana.com/address/8HVDihB8NrYpqzRNrFuiSPUC7B4rRqa8HFRJNWoxH4JK?cluster=devnet)

## 📞 Contact

For questions or support, please open an issue in this repository.
