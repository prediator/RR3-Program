use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount, Transfer, Mint};

declare_id!("8HVDihB8NrYpqzRNrFuiSPUC7B4rRqa8HFRJNWoxH4JK");

#[program]
pub mod staker {
    use super::*;

    // REPLACE ADDRESS of RR3 mint by running solana address -k keys_for_test/rr3_mint_localhost.json
    pub const RR3_MINT_ADDRESS: &str = "Enpkrq4rZwT8mCjk6fmqaH9pLU3iwrDzf1772Ct1eLqx";
    
    // New 3.33% Commission Structure (basis points)
    pub const TOTAL_COMMISSION_BPS: u64 = 333; // 3.33% total commission on staking (taken from staked RR3)
    pub const FEE_WALLET_BPS: u64 = 300; // 3.00% fee wallet
    pub const EXPENSE_WALLET_BPS: u64 = 33; // 0.33% expense wallet
    pub const MARKETING_WALLET_BPS: u64 = 3; // 0.03% marketing wallet
    // Note: Burn wallet commission removed (was 0.003% - too small for basis points)
    
    // Rewards are distributed in RR3 tokens (not SOL)
    
    // Professional lock periods only - no short-term staking allowed
    
    // Predefined lock periods for professional staking
    pub const LOCK_3_MONTHS: i64 = 3 * 30 * 24 * 60 * 60; // 3 months in seconds (7,776,000)
    pub const LOCK_6_MONTHS: i64 = 6 * 30 * 24 * 60 * 60; // 6 months in seconds (15,552,000) 
    pub const LOCK_1_YEAR: i64 = 365 * 24 * 60 * 60; // 1 year in seconds (31,536,000)
    pub const LOCK_2_YEARS: i64 = 2 * 365 * 24 * 60 * 60; // 2 years in seconds (63,072,000)
    pub const LOCK_3_YEARS: i64 = 3 * 365 * 24 * 60 * 60; // 3 years in seconds (94,608,000)

    // Final reward multipliers (basis points). These determine total payout as:
    // final_payout = principal * multiplier / 100
    // e.g. 105 => 5% reward on top of principal
    pub const REWARD_MULTIPLIER_3_MONTHS: u64 = 105; // 5% total payout
    pub const REWARD_MULTIPLIER_6_MONTHS: u64 = 112; // 12% total payout
    pub const REWARD_MULTIPLIER_1_YEAR: u64 = 130;   // 30% total payout
    pub const REWARD_MULTIPLIER_2_YEARS: u64 = 170;  // 70% total payout
    pub const REWARD_MULTIPLIER_3_YEARS: u64 = 250;  // 150% total payout



    // Initialize global distribution state (admin only)
    pub fn initialize_distribution_state(
        _ctx: Context<InitializeDistributionState>
    ) -> Result<()> {
        let distribution_state = &mut _ctx.accounts.distribution_state;
        let clock = Clock::get()?;
        
        distribution_state.admin = _ctx.accounts.admin.key();
        distribution_state.last_distribution_time = clock.unix_timestamp;
        distribution_state.total_rr3_staked = 0;
        distribution_state.monthly_rr3_for_rewards = 0;
        distribution_state.monthly_expense_fees = 0;
        distribution_state.monthly_marketing_fees = 0;
        distribution_state.monthly_burn_fees = 0;
        distribution_state.distribution_round = 0;
        
        msg!("Distribution state initialized - rewards will be distributed in RR3 tokens");
        Ok(())
    }

    // Admin function: Record monthly RR3 allocation for reward distribution
    pub fn record_monthly_collection(
        ctx: Context<RecordMonthlyCollection>,
        total_rr3_for_rewards: u64, // Total RR3 tokens allocated for rewards (in RR3 token units)
    ) -> Result<()> {
        let distribution_state = &mut ctx.accounts.distribution_state;
        let clock = Clock::get()?;
        
        // Check if it's been at least 5 minutes since last distribution
        let time_since_last = clock.unix_timestamp - distribution_state.last_distribution_time;
        require!(
            time_since_last >= (5 * 60) as i64, // 5 minutes instead of 30 days
            StakeError::TooEarlyForDistribution
        );

        // Calculate commission breakdown from RR3 allocated (3.33% total of staked amount stays as commission)
        let fee_wallet_amount = (total_rr3_for_rewards * FEE_WALLET_BPS) / TOTAL_COMMISSION_BPS;
        let expense_wallet_amount = (total_rr3_for_rewards * EXPENSE_WALLET_BPS) / TOTAL_COMMISSION_BPS;
        let marketing_wallet_amount = (total_rr3_for_rewards * MARKETING_WALLET_BPS) / TOTAL_COMMISSION_BPS;
        let burn_wallet_amount = total_rr3_for_rewards.saturating_sub(fee_wallet_amount + expense_wallet_amount + marketing_wallet_amount);

        distribution_state.monthly_rr3_for_rewards = fee_wallet_amount; // 3.00% fee wallet
        distribution_state.monthly_expense_fees = expense_wallet_amount; // 0.33% expense wallet
        distribution_state.monthly_marketing_fees = marketing_wallet_amount; // 0.03% marketing wallet
        distribution_state.monthly_burn_fees = burn_wallet_amount;
        distribution_state.distribution_round += 1;
        distribution_state.last_distribution_time = clock.unix_timestamp; // Update timestamp for next check
        
        msg!("Monthly RR3 rewards recorded: {} RR3 tokens total allocated", 
            total_rr3_for_rewards as f64 / 100_000_000.0
        );
        msg!("Fee wallet: {} RR3, Expense: {} RR3, Marketing: {} RR3, Burn: {} RR3", 
            fee_wallet_amount as f64 / 100_000_000.0,
            expense_wallet_amount as f64 / 100_000_000.0,
            marketing_wallet_amount as f64 / 100_000_000.0,
            burn_wallet_amount as f64 / 100_000_000.0
        );
        msg!("Distribution round: {}", distribution_state.distribution_round);
        
        Ok(())
    }

    // Admin function: Update total staked amount for current distribution (supports weighted staking)
    // Note: For weighted rewards, this should be the sum of (amount * lock_multiplier / 100) for all stakes
    pub fn update_total_staked(
        ctx: Context<UpdateTotalStaked>,
        total_weighted_staked: u64,
    ) -> Result<()> {
        let distribution_state = &mut ctx.accounts.distribution_state;
        
        distribution_state.total_rr3_staked = total_weighted_staked;
        
        msg!("Updated total weighted RR3 staked: {} tokens", total_weighted_staked as f64 / 100_000_000.0);
        msg!("Note: This should be the sum of weighted stakes (amount * multiplier) for accurate distribution");
        Ok(())
    }

    // Calculate and assign monthly RR3 rewards to individual staker (weighted by lock duration)
    pub fn assign_monthly_rewards(
        ctx: Context<AssignMonthlyRewards>,
        stake_index: u32,
    ) -> Result<()> {
        let distribution_state = &ctx.accounts.distribution_state;
        let stake_record = &mut ctx.accounts.stake_record;
        let clock = Clock::get()?;
        
        require!(
            distribution_state.monthly_rr3_for_rewards > 0,
            StakeError::NoMonthlyCollection
        );
        require!(
            distribution_state.total_rr3_staked > 0,
            StakeError::NoStakersForDistribution
        );
        
        // Prevent double-rewarding: ensure stake hasn't already received rewards for this round
        require!(
            stake_record.last_distribution_round < distribution_state.distribution_round,
            StakeError::AlreadyReceivedRewardsThisRound
        );

        // Calculate lock duration multiplier (basis points for precision)
        // Base: 100 = 1.0x, higher values = minimal bonus rewards (conservative approach)
        let lock_multiplier = match stake_record.lock_duration {
            LOCK_3_MONTHS => 100,  // 1.0000x - base multiplier
            LOCK_6_MONTHS => 101,  // 1.0100x - 1% bonus
            LOCK_1_YEAR => 102,    // 1.0200x - 2% bonus
            LOCK_2_YEARS => 103,   // 1.0300x - 3% bonus
            LOCK_3_YEARS => 103,   // 1.0333x - 3.33% bonus (rounded to 103)
            _ => {
                // For development/testing periods (< 3 months), use proportional multiplier
                let months = stake_record.lock_duration / (30 * 24 * 60 * 60);
                if months == 0 {
                    50  // Less than 1 month: 0.5x
                } else {
                    (100 + (months as u64 * 8)).min(100) // Proportional up to 3 months
                }
            }
        };

        msg!("Lock duration: {} seconds, Multiplier: {}x", 
            stake_record.lock_duration, 
            lock_multiplier as f64 / 100.0
        );

        // Calculate weighted stake amount
        let weighted_amount = (stake_record.amount as u128)
            .checked_mul(lock_multiplier as u128).unwrap()
            .checked_div(100).unwrap();

        // Calculate weighted total staked (NOTE: This needs to be updated in update_total_staked to include weights)
        // For now, we use the existing total and apply the same logic
        let weighted_total = distribution_state.total_rr3_staked as u128;

        // Calculate proportional reward: (weighted_stake / weighted_total) * monthly_rr3_rewards
        let user_share = weighted_amount
            .checked_mul(distribution_state.monthly_rr3_for_rewards as u128).unwrap()
            .checked_div(weighted_total).unwrap();

        let reward_amount = user_share as u64;

        // Add to pending rewards
        stake_record.pending_rr3_rewards += reward_amount;
        stake_record.last_distribution_round = distribution_state.distribution_round;
        
        msg!("Assigned {} RR3 rewards to staker {} (stake: {}, weighted: {}, multiplier: {}x)",
            reward_amount as f64 / 100_000_000.0,
            stake_record.user,
            stake_record.amount as f64 / 100_000_000.0,
            weighted_amount as f64 / 100_000_000.0,
            lock_multiplier as f64 / 100.0
        );
        
        Ok(())
    }


    // Admin function: Update total staked amount for current distribution (supports weighted staking)

    // Admin function: Transfer marketing fees to marketing wallet
    pub fn transfer_marketing_fees(
        ctx: Context<TransferMarketingFees>,
    ) -> Result<()> {
        let distribution_state = &mut ctx.accounts.distribution_state;
        
        require!(
            distribution_state.monthly_marketing_fees > 0,
            StakeError::NoMarketingFeesToTransfer
        );

        let marketing_amount = distribution_state.monthly_marketing_fees;

        // Transfer SOL from treasury to marketing wallet
        **ctx.accounts.treasury_sol_account.lamports.borrow_mut() -= marketing_amount;
        **ctx.accounts.marketing_wallet_sol_account.lamports.borrow_mut() += marketing_amount;

        // Reset marketing fees to 0
        distribution_state.monthly_marketing_fees = 0;
        
        msg!("Transferred {} SOL to marketing wallet", marketing_amount as f64 / 1_000_000_000.0);
        Ok(())
    }

    // Admin function: Transfer fee wallet funds (3.00%)
    pub fn transfer_fee_wallet(
        ctx: Context<TransferFeeWallet>,
    ) -> Result<()> {
        let distribution_state = &mut ctx.accounts.distribution_state;
        
        require!(
            distribution_state.monthly_rr3_for_rewards > 0,
            StakeError::NoFeeWalletFeesToTransfer
        );

        let fee_wallet_amount = distribution_state.monthly_rr3_for_rewards;

        // Transfer SOL from treasury to fee wallet
        **ctx.accounts.treasury_sol_account.lamports.borrow_mut() -= fee_wallet_amount;
        **ctx.accounts.fee_wallet_sol_account.lamports.borrow_mut() += fee_wallet_amount;

        // Reset fee wallet amount to 0
        distribution_state.monthly_rr3_for_rewards = 0;
        
        msg!("Transferred {} SOL to fee wallet", fee_wallet_amount as f64 / 1_000_000_000.0);
        Ok(())
    }

    // Admin function: Burn fees (transfer SOL to burn wallet)
    pub fn burn_fees(
        ctx: Context<BurnFees>,
    ) -> Result<()> {
        let distribution_state = &mut ctx.accounts.distribution_state;
        
        require!(
            distribution_state.monthly_burn_fees > 0,
            StakeError::NoBurnFeesToTransfer
        );

        let burn_amount = distribution_state.monthly_burn_fees;

        // Transfer SOL to burn wallet (or implement actual burn if preferred)
        **ctx.accounts.treasury_sol_account.lamports.borrow_mut() -= burn_amount;
        **ctx.accounts.burn_wallet_sol_account.lamports.borrow_mut() += burn_amount;

        // Reset burn fees to 0
        distribution_state.monthly_burn_fees = 0;
        
        msg!("Transferred {} SOL to burn wallet", burn_amount as f64 / 1_000_000_000.0);
        Ok(())
    }

    // Helper function to get lock period description
    pub fn get_lock_period_info(
        ctx: Context<GetLockPeriodInfo>,
        lock_duration_seconds: i64,
    ) -> Result<()> {
        let (period_name, multiplier) = match lock_duration_seconds {
            LOCK_3_MONTHS => ("3 months", 100),
            LOCK_6_MONTHS => ("6 months", 125), 
            LOCK_1_YEAR => ("1 year", 150),
            LOCK_2_YEARS => ("2 years", 200),
            LOCK_3_YEARS => ("3 years", 300),
            _ => ("Invalid lock period", 0),
        };
        
        msg!("Lock period: {} ({} seconds)", period_name, lock_duration_seconds);
        msg!("Reward multiplier: {}x", multiplier as f64 / 100.0);
        msg!("Available periods:");
        msg!("  - 3 months: 1.0x multiplier");
        msg!("  - 6 months: 1.25x multiplier (+25% rewards)");
        msg!("  - 1 year: 1.5x multiplier (+50% rewards)");
        msg!("  - 2 years: 2.0x multiplier (+100% rewards)");
        msg!("  - 3 years: 3.0x multiplier (+200% rewards)");
        Ok(())
    }

    // Admin function: Complete monthly distribution (reset for next month)
    pub fn complete_monthly_distribution(
        ctx: Context<CompleteMonthlyDistribution>,
    ) -> Result<()> {
        let distribution_state = &mut ctx.accounts.distribution_state;
        let clock = Clock::get()?;
        
        distribution_state.last_distribution_time = clock.unix_timestamp;
        distribution_state.monthly_rr3_for_rewards = 0;
        
        msg!("Monthly distribution completed for round {}", distribution_state.distribution_round);
        Ok(())
    }
 

    pub fn create_rr3_token_bag(
        _ctx: Context<CreateRR3TokenBag>
    ) -> Result<()> {
        msg!("RR3 Staking Bag created");
        Ok(())
    }

    // Create treasury bag for reward distribution
    pub fn create_treasury_bag(
        _ctx: Context<CreateTreasuryBag>
    ) -> Result<()> {
        msg!("RR3 Treasury Bag created for reward distribution");
        Ok(())
    }

    // Create buyback treasury for emergency buyback guarantee
    pub fn create_buyback_treasury(
        _ctx: Context<CreateBuybackTreasury>
    ) -> Result<()> {
        msg!("Buyback treasury created for emergency buyback guarantee");
        Ok(())
    }

    // New stake function - only locks tokens, no immediate minting
    // Stake tokens (supports multiple stakes per user)
    pub fn stake(
        ctx: Context<Stake>,
        _program_rr3_bag_bump: u8,
        stake_index: u32,
        rr3_amount: u64,
        lock_duration_seconds: i64,
    ) -> Result<()> {
        let stake_record = &mut ctx.accounts.stake_record;
        let user_stake_counter = &mut ctx.accounts.user_stake_counter;
        let clock = Clock::get()?;

        msg!("Creating stake #{} with {} RR3 tokens and {} seconds lock", stake_index, rr3_amount, lock_duration_seconds);

        // Validate lock duration - allow development testing periods
        let is_professional_period = lock_duration_seconds == LOCK_3_MONTHS ||
            lock_duration_seconds == LOCK_6_MONTHS ||
            lock_duration_seconds == LOCK_1_YEAR ||
            lock_duration_seconds == LOCK_2_YEARS ||
            lock_duration_seconds == LOCK_3_YEARS;
        
        let is_development_period = lock_duration_seconds >= 300; // Allow 5+ minutes for testing
        
        require!(
            is_professional_period || is_development_period,
            StakeError::InvalidLockPeriod
        );

        // Initialize user counter if this is the first time
        if user_stake_counter.user == Pubkey::default() {
            user_stake_counter.user = ctx.accounts.user_rr3_token_bag_authority.key();
            user_stake_counter.total_stakes = 0;
            user_stake_counter.active_stakes = 0;
        }

        // Ensure stake index matches the expected next index
        require!(
            stake_index == user_stake_counter.total_stakes,
            StakeError::InvalidStakeIndex
        );

        // Transfer full RR3 amount from user to program's staking bag (no commission deducted from RR3)
        let cpi_ctx = CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            token::Transfer {
                from: ctx.accounts.user_rr3_token_bag.to_account_info(),
                authority: ctx.accounts.user_rr3_token_bag_authority.to_account_info(),
                to: ctx.accounts.program_rr3_token_bag.to_account_info(),
            }
        );
        token::transfer(cpi_ctx, rr3_amount)?;

        // Calculate RR3 commission (3.33% of staked amount stays in program treasury)
        let commission_amount = (rr3_amount * TOTAL_COMMISSION_BPS) / 10000;
        let net_stake_amount = rr3_amount - commission_amount;
        
        msg!("RR3 commission: {} tokens (3.33%)", commission_amount);
        msg!("Net stake amount: {} RR3 tokens", net_stake_amount);

        // Record the stake (net amount after commission)
        stake_record.user = ctx.accounts.user_rr3_token_bag_authority.key();
        stake_record.stake_index = stake_index;
        stake_record.amount = net_stake_amount;
        stake_record.stake_time = clock.unix_timestamp;
        stake_record.lock_duration = lock_duration_seconds;
        stake_record.unlock_time = clock.unix_timestamp + lock_duration_seconds;
        stake_record.last_reward_claim = clock.unix_timestamp;
        stake_record.pending_rr3_rewards = 0;
        stake_record.total_rr3_claimed = 0;
        stake_record.last_distribution_round = 0;

        // Update user counter
        user_stake_counter.total_stakes += 1;
        user_stake_counter.active_stakes += 1;

        msg!("Stake #{} recorded successfully. Unlocks at: {}", stake_index, stake_record.unlock_time);
        Ok(())
    }

    // Add to existing stake
    pub fn add_stake(
        ctx: Context<AddStake>,
        _program_rr3_bag_bump: u8,
        rr3_amount: u64,
        lock_duration_seconds: i64,
    ) -> Result<()> {
        let stake_record = &mut ctx.accounts.stake_record;
        let clock = Clock::get()?;

        msg!("Adding {} RR3 tokens to existing stake with {} seconds lock", rr3_amount, lock_duration_seconds);

        // Validate lock duration - allow development testing periods
        let is_professional_period = lock_duration_seconds == LOCK_3_MONTHS ||
            lock_duration_seconds == LOCK_6_MONTHS ||
            lock_duration_seconds == LOCK_1_YEAR ||
            lock_duration_seconds == LOCK_2_YEARS ||
            lock_duration_seconds == LOCK_3_YEARS;
        
        let is_development_period = lock_duration_seconds >= 300; // Allow 5+ minutes for testing
        
        require!(
            is_professional_period || is_development_period,
            StakeError::InvalidLockPeriod
        );

        // Transfer RR3 from user to program's staking bag
        let cpi_ctx = CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            token::Transfer {
                from: ctx.accounts.user_rr3_token_bag.to_account_info(),
                authority: ctx.accounts.user_rr3_token_bag_authority.to_account_info(),
                to: ctx.accounts.program_rr3_token_bag.to_account_info(),
            }
        );
        token::transfer(cpi_ctx, rr3_amount)?;

        // Add to existing stake and extend lock duration if longer
        stake_record.amount += rr3_amount;
        
        // If new lock duration is longer, extend the unlock time
        let new_unlock_time = clock.unix_timestamp + lock_duration_seconds;
        if new_unlock_time > stake_record.unlock_time {
            stake_record.lock_duration = lock_duration_seconds;
            stake_record.unlock_time = new_unlock_time;
        }
        
        msg!("Added {} RR3 to existing stake. New total: {}", rr3_amount, stake_record.amount);
        Ok(())
    }

    // Claim RR3 rewards from monthly distributions
    // Users can claim monthly rewards ANYTIME (even during lock period)
    // This allows them to withdraw earned rewards before unstaking
    pub fn claim_rr3_rewards(
        ctx: Context<ClaimRR3Rewards>,
        stake_index: u32,
        _program_rr3_bag_bump: u8,
    ) -> Result<()> {
        let stake_record = &mut ctx.accounts.stake_record;
        
        msg!("Claiming RR3 rewards for stake #{}", stake_index);

        let pending_rewards = stake_record.pending_rr3_rewards;
        require!(pending_rewards > 0, StakeError::NoRewardsAvailable);

        // Transfer RR3 tokens from program treasury to user
        let seeds = &[
            b"token_bag".as_ref(),
            &[_program_rr3_bag_bump],
        ];
        let signer = &[&seeds[..]];
        
        let cpi_ctx = CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            token::Transfer {
                from: ctx.accounts.program_rr3_token_bag.to_account_info(),
                authority: ctx.accounts.program_rr3_token_bag.to_account_info(),
                to: ctx.accounts.user_rr3_token_bag.to_account_info(),
            },
            signer
        );
        token::transfer(cpi_ctx, pending_rewards)?;

        // Update stake record
        stake_record.pending_rr3_rewards = 0;
        stake_record.total_rr3_claimed += pending_rewards;
        stake_record.last_reward_claim = Clock::get()?.unix_timestamp;
        
        msg!("Claimed {} RR3 as rewards", pending_rewards as f64 / 100_000_000.0);
        Ok(())
    }

    // Unstake principal + any pending RR3 rewards  
    pub fn unstake(
        ctx: Context<UnStake>,
        program_rr3_bag_bump: u8,
        stake_index: u32,
    ) -> Result<()> {
        let stake_record = &ctx.accounts.stake_record;
        let user_stake_counter = &mut ctx.accounts.user_stake_counter;
        let clock = Clock::get()?;

        msg!("Unstaking stake #{} with {} RR3 tokens", stake_index, stake_record.amount);

        let pending_rr3_rewards = stake_record.pending_rr3_rewards;
        let is_fully_unlocked = clock.unix_timestamp >= stake_record.unlock_time;

        // Determine payout based on unlock status
        let (final_bonus, monthly_rewards_to_return): (u64, u64) = if is_fully_unlocked {
            // NORMAL UNLOCK: Full completion after lock period ends
            // User gets: principal + completion bonus + all monthly rewards earned
            let final_multiplier_bps: u64 = match stake_record.lock_duration {
                LOCK_3_MONTHS => REWARD_MULTIPLIER_3_MONTHS,
                LOCK_6_MONTHS => REWARD_MULTIPLIER_6_MONTHS,
                LOCK_1_YEAR => REWARD_MULTIPLIER_1_YEAR,
                LOCK_2_YEARS => REWARD_MULTIPLIER_2_YEARS,
                LOCK_3_YEARS => REWARD_MULTIPLIER_3_YEARS,
                _ => {
                    // For non-standard lock periods, no completion bonus
                    // Only professional lock periods (3mo, 6mo, 1yr, 2yr, 3yr) get bonuses
                    100 // 1.0x - return only principal, no bonus
                }
            };

            // final payout = principal * multiplier / 100
            let final_payout = ((stake_record.amount as u128)
                .checked_mul(final_multiplier_bps as u128).unwrap()
                .checked_div(100).unwrap()) as u64;

            // bonus = final_payout - principal
            let completion_bonus = final_payout.saturating_sub(stake_record.amount);
            
            (completion_bonus, pending_rr3_rewards)
        } else {
            // EMERGENCY UNLOCK: Early exit before lock period ends
            // User gets: principal + ALL earned monthly rewards (completed cycles)
            // User FORFEITS: ONLY the completion bonus (requires full lock period)
            // 
            // Rationale: If user completed months 1-3 and emergency unstakes in month 4:
            //   ✅ Gets month 1, 2, 3 rewards (earned through completed cycles)
            //   ❌ Forfeits completion bonus (didn't complete full lock period)
            msg!("Emergency Unlock activated - returning principal + earned monthly rewards");
            msg!("Returning: {} RR3 in earned monthly rewards", 
                pending_rr3_rewards as f64 / 100_000_000.0
            );
            msg!("Forfeited: Completion bonus only (requires full lock period)");
            
            (0, pending_rr3_rewards)  // No completion bonus, but gets earned monthly rewards
        };

        // Total to return based on unlock type
        let total_rr3_to_return = stake_record.amount  // Net staked principal (always returned)
            .saturating_add(final_bonus)              // Completion bonus (only if fully unlocked)
            .saturating_add(monthly_rewards_to_return); // Monthly rewards (only if fully unlocked)

        // Transfer principal + rewards back to user from staking bag
        let rr3_mint_address = ctx.accounts.rr3_mint.key();
        let seeds = &[rr3_mint_address.as_ref(), &[program_rr3_bag_bump]];
        let signer = [&seeds[..]];

        let cpi_ctx = CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            token::Transfer {
                from: ctx.accounts.program_rr3_token_bag.to_account_info(),
                authority: ctx.accounts.program_rr3_token_bag.to_account_info(),
                to: ctx.accounts.user_rr3_token_bag.to_account_info()
            },
            &signer
        );
        token::transfer(cpi_ctx, total_rr3_to_return)?;

        // Update user stake counter
        user_stake_counter.active_stakes = user_stake_counter.active_stakes.saturating_sub(1);

        if is_fully_unlocked {
            msg!("✓ Normal Unlock - Stake #{} completed after full lock period", stake_index);
            msg!("  Principal: {} RR3", stake_record.amount as f64 / 100_000_000.0);
            msg!("  Completion bonus: {} RR3", final_bonus as f64 / 100_000_000.0);
            msg!("  Monthly rewards: {} RR3", monthly_rewards_to_return as f64 / 100_000_000.0);
            msg!("  TOTAL PAYOUT: {} RR3", total_rr3_to_return as f64 / 100_000_000.0);
        } else {
            msg!("⚠ Emergency Unlock - Stake #{} exited early (before lock period completion)", stake_index);
            msg!("  Net principal returned: {} RR3", stake_record.amount as f64 / 100_000_000.0);
            msg!("  Earned monthly rewards returned: {} RR3", monthly_rewards_to_return as f64 / 100_000_000.0);
            msg!("  Forfeited completion bonus: 0 RR3 (requires full lock period)");
            msg!("  TOTAL PAYOUT: {} RR3 (principal + earned monthly rewards)", total_rr3_to_return as f64 / 100_000_000.0);
        }
        Ok(())
    }

    // Check if a staker is eligible for monthly rewards with weighted multiplier info
    pub fn check_eligibility(
        ctx: Context<CheckEligibility>,
        stake_index: u32,
    ) -> Result<()> {
        let stake_record = &ctx.accounts.stake_record;
        let clock = Clock::get()?;
        
        let staking_duration = clock.unix_timestamp - stake_record.stake_time;
        let days_staked = staking_duration / (24 * 60 * 60);
        let time_until_unlock = stake_record.unlock_time - clock.unix_timestamp;
        let is_unlocked = clock.unix_timestamp >= stake_record.unlock_time;
        
        // Calculate reward multiplier
        let lock_multiplier = match stake_record.lock_duration {
            LOCK_3_MONTHS => 100,
            LOCK_6_MONTHS => 101,
            LOCK_1_YEAR => 102,
            LOCK_2_YEARS => 103,
            LOCK_3_YEARS => 103,
            _ => {
                let months = stake_record.lock_duration / (30 * 24 * 60 * 60);
                if months == 0 { 50 } else { (100 + (months as u64 * 8)).min(100) }
            }
        };
        
        let weighted_amount = (stake_record.amount as u128)
            .checked_mul(lock_multiplier as u128).unwrap()
            .checked_div(100).unwrap();
        
        msg!("Staker: {}", stake_record.user);
        msg!("Staked amount: {} RR3", stake_record.amount as f64 / 100_000_000.0);
        msg!("Weighted amount: {} RR3 ({}x multiplier)", 
            weighted_amount as f64 / 100_000_000.0,
            lock_multiplier as f64 / 100.0
        );
        msg!("Days staked: {}", days_staked);
        msg!("Lock duration: {} seconds", stake_record.lock_duration);
        msg!("Unlocked: {}", is_unlocked);
        msg!("Time until unlock: {} seconds", if is_unlocked { 0 } else { time_until_unlock });
        msg!("Pending RR3 rewards: {} RR3", stake_record.pending_rr3_rewards as f64 / 100_000_000.0);
        
        Ok(())
    }
}

// Stake record account to track individual stakes
#[account]
pub struct StakeRecord {
    pub user: Pubkey,           // 32 bytes
    pub stake_index: u32,       // 4 bytes - Index of this stake for the user (0, 1, 2, etc.)
    pub amount: u64,            // 8 bytes - RR3 tokens staked
    pub stake_time: i64,        // 8 bytes
    pub lock_duration: i64,     // 8 bytes - Lock duration in seconds
    pub unlock_time: i64,       // 8 bytes - Unix timestamp when tokens unlock
    pub last_reward_claim: i64, // 8 bytes
    pub pending_rr3_rewards: u64, // 8 bytes - Claimable RR3 rewards (in RR3 token units)
    pub total_rr3_claimed: u64,   // 8 bytes - Total RR3 rewards claimed
    pub last_distribution_round: u64, // 8 bytes - Last distribution round participated in
}

// New: User stake counter to track number of stakes per user
#[account]
pub struct UserStakeCounter {
    pub user: Pubkey,           // 32 bytes
    pub total_stakes: u32,      // 4 bytes - Total number of stakes created by this user
    pub active_stakes: u32,     // 4 bytes - Number of currently active stakes
}



// Global distribution state account
#[account]
pub struct DistributionState {
    pub admin: Pubkey,                    // 32 bytes
    pub last_distribution_time: i64,      // 8 bytes - Unix timestamp of last distribution
    pub total_rr3_staked: u64,           // 8 bytes - Total RR3 staked across all users
    pub monthly_rr3_for_rewards: u64,     // 8 bytes - RR3 tokens allocated for monthly reward distribution
    pub monthly_expense_fees: u64,        // 8 bytes - Expense wallet amount (0.33%)
    pub monthly_marketing_fees: u64,      // 8 bytes - Marketing wallet amount (0.03%)
    pub monthly_burn_fees: u64,           // 8 bytes - Burn wallet amount (0.003%)
    pub distribution_round: u64,          // 8 bytes - Current distribution round number
}



// Initialize distribution state context
#[derive(Accounts)]
pub struct InitializeDistributionState<'info> {
    #[account(
        init,
        payer = admin,
        space = 8 + 32 + 8 + 8 + 8 + 8 + 8 + 8 + 8, // discriminator + pubkey + 7 u64s/i64s (added marketing and burn fees)
        seeds = [b"distribution_state"],
        bump,
    )]
    pub distribution_state: Account<'info, DistributionState>,

    #[account(mut)]
    pub admin: Signer<'info>,

    pub system_program: Program<'info, System>,
}

// Record monthly collection context
#[derive(Accounts)]
pub struct RecordMonthlyCollection<'info> {
    #[account(
        mut,
        seeds = [b"distribution_state"],
        bump,
        has_one = admin,
    )]
    pub distribution_state: Account<'info, DistributionState>,

    pub admin: Signer<'info>,
}

// Update total staked context
#[derive(Accounts)]
pub struct UpdateTotalStaked<'info> {
    #[account(
        mut,
        seeds = [b"distribution_state"],
        bump,
        has_one = admin,
    )]
    pub distribution_state: Account<'info, DistributionState>,

    pub admin: Signer<'info>,
}

// Assign monthly rewards context
#[derive(Accounts)]
#[instruction(stake_index: u32)]
pub struct AssignMonthlyRewards<'info> {
    #[account(
        seeds = [b"distribution_state"],
        bump,
        has_one = admin,
    )]
    pub distribution_state: Account<'info, DistributionState>,

    #[account(
        mut,
        seeds = [b"stake", staker.key().as_ref(), stake_index.to_le_bytes().as_ref()],
        bump,
    )]
    pub stake_record: Account<'info, StakeRecord>,

    /// CHECK: The staker's public key for PDA derivation
    pub staker: AccountInfo<'info>,

    pub admin: Signer<'info>,
}

// Transfer expense fees context
#[derive(Accounts)]
pub struct TransferExpenseFees<'info> {
    #[account(
        mut,
        seeds = [b"distribution_state"],
        bump,
        has_one = admin,
    )]
    pub distribution_state: Account<'info, DistributionState>,

    #[account(
        mut,
        seeds = [b"treasury_sol"],
        bump,
    )]
    /// CHECK: This is the treasury SOL account PDA
    pub treasury_sol_account: AccountInfo<'info>,

    #[account(mut)]
    /// CHECK: This is the expense wallet SOL account
    pub expense_wallet_sol_account: AccountInfo<'info>,

    pub admin: Signer<'info>,

    pub system_program: Program<'info, System>,
}

// Transfer marketing fees context
#[derive(Accounts)]
pub struct TransferMarketingFees<'info> {
    #[account(
        mut,
        seeds = [b"distribution_state"],
        bump,
        has_one = admin,
    )]
    pub distribution_state: Account<'info, DistributionState>,

    #[account(
        mut,
        seeds = [b"treasury_sol"],
        bump,
    )]
    /// CHECK: This is the treasury SOL account PDA
    pub treasury_sol_account: AccountInfo<'info>,

    #[account(mut)]
    /// CHECK: This is the marketing wallet SOL account
    pub marketing_wallet_sol_account: AccountInfo<'info>,

    pub admin: Signer<'info>,

    pub system_program: Program<'info, System>,
}

// Transfer fee wallet context
#[derive(Accounts)]
pub struct TransferFeeWallet<'info> {
    #[account(
        mut,
        seeds = [b"distribution_state"],
        bump,
        has_one = admin,
    )]
    pub distribution_state: Account<'info, DistributionState>,

    #[account(
        mut,
        seeds = [b"treasury_sol"],
        bump,
    )]
    /// CHECK: This is the treasury SOL account PDA
    pub treasury_sol_account: AccountInfo<'info>,

    #[account(mut)]
    /// CHECK: This is the fee wallet SOL account
    pub fee_wallet_sol_account: AccountInfo<'info>,

    pub admin: Signer<'info>,

    pub system_program: Program<'info, System>,
}

// Burn fees context
#[derive(Accounts)]
pub struct BurnFees<'info> {
    #[account(
        mut,
        seeds = [b"distribution_state"],
        bump,
        has_one = admin,
    )]
    pub distribution_state: Account<'info, DistributionState>,

    #[account(
        mut,
        seeds = [b"treasury_sol"],
        bump,
    )]
    /// CHECK: This is the treasury SOL account PDA
    pub treasury_sol_account: AccountInfo<'info>,

    #[account(mut)]
    /// CHECK: This is the burn wallet SOL account
    pub burn_wallet_sol_account: AccountInfo<'info>,

    pub admin: Signer<'info>,

    pub system_program: Program<'info, System>,
}

// Emergency buyback context
#[derive(Accounts)]
#[instruction(stake_index: u32)]
pub struct EmergencyBuyback<'info> {
    #[account(
        mut,
        seeds = [b"stake", user_authority.key().as_ref(), stake_index.to_le_bytes().as_ref()],
        bump,
        close = user_authority, // Close the account and refund rent
    )]
    pub stake_record: Account<'info, StakeRecord>,

    #[account(
        mut,
        seeds = [b"user_counter", user_authority.key().as_ref()],
        bump,
    )]
    pub user_stake_counter: Account<'info, UserStakeCounter>,

    #[account(mut)]
    pub user_rr3_token_bag: Account<'info, TokenAccount>,

    pub user_authority: Signer<'info>,

    #[account(
        mut,
        seeds = [b"buyback_treasury"],
        bump,
    )]
    pub buyback_treasury_bag: Account<'info, TokenAccount>,

    #[account(
        address = RR3_MINT_ADDRESS.parse::<Pubkey>().unwrap(),
    )]
    pub rr3_mint: Account<'info, Mint>,

    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

// User burn tokens context
#[derive(Accounts)]
pub struct UserBurnTokens<'info> {
    #[account(mut)]
    pub user_rr3_token_bag: Account<'info, TokenAccount>,

    pub user_authority: Signer<'info>,

    #[account(mut)]
    pub burn_wallet_token_bag: Account<'info, TokenAccount>,

    #[account(
        address = RR3_MINT_ADDRESS.parse::<Pubkey>().unwrap(),
    )]
    pub rr3_mint: Account<'info, Mint>,

    pub token_program: Program<'info, Token>,
}

// Complete monthly distribution context
#[derive(Accounts)]
pub struct CompleteMonthlyDistribution<'info> {
    #[account(
        mut,
        seeds = [b"distribution_state"],
        bump,
        has_one = admin,
    )]
    pub distribution_state: Account<'info, DistributionState>,

    pub admin: Signer<'info>,
}

// SOL Treasury account context
#[derive(Accounts)]
pub struct CreateSOLTreasury<'info> {
    #[account(
        init,
        payer = admin,
        space = 8, // Just a simple account to hold SOL
        seeds = [b"treasury_sol"],
        bump,
    )]
    /// CHECK: This is the SOL treasury PDA account
    pub treasury_sol_account: AccountInfo<'info>,

    #[account(mut)]
    pub admin: Signer<'info>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(program_rr3_bag_bump: u8, stake_index: u32)]
pub struct Stake<'info> {
    // Minimized for stack space - core accounts only
    pub token_program: Program<'info, Token>,

    #[account(mut)]
    pub user_rr3_token_bag: Account<'info, TokenAccount>,

    #[account(mut)]
    pub user_rr3_token_bag_authority: Signer<'info>,

    #[account(
        mut,
        seeds = [ rr3_mint.key().as_ref() ],
        bump = program_rr3_bag_bump,
    )]
    pub program_rr3_token_bag: Account<'info, TokenAccount>,

    #[account(
        address = RR3_MINT_ADDRESS.parse::<Pubkey>().unwrap(),
    )]
    pub rr3_mint: Account<'info, Mint>,

    #[account(
        init,
        payer = user_rr3_token_bag_authority,
        space = 108, // 8 + 32 + 4 + 8*8 = 108 bytes
        seeds = [b"stake", user_rr3_token_bag_authority.key().as_ref(), stake_index.to_le_bytes().as_ref()],
        bump,
    )]
    pub stake_record: Account<'info, StakeRecord>,

    #[account(
        init_if_needed,
        payer = user_rr3_token_bag_authority,
        space = 48, // 8 + 32 + 4 + 4 = 48 bytes
        seeds = [b"user_counter", user_rr3_token_bag_authority.key().as_ref()],
        bump,
    )]
    pub user_stake_counter: Account<'info, UserStakeCounter>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(program_rr3_bag_bump: u8)]
pub struct AddStake<'info> {
    // SPL Token Program
    pub token_program: Program<'info, Token>,

    // Associated Token Account for User which holds RR3.
    #[account(mut)]
    pub user_rr3_token_bag: Account<'info, TokenAccount>,

    // The authority allowed to mutate the above ⬆️
    #[account(mut)]
    pub user_rr3_token_bag_authority: Signer<'info>,

    // Used to receive RR3 from users (staking bag)
    #[account(
        mut,
        seeds = [ rr3_mint.key().as_ref() ],
        bump = program_rr3_bag_bump,
    )]
    pub program_rr3_token_bag: Account<'info, TokenAccount>,

    // Required for the PDA above ⬆️
    #[account(
        address = RR3_MINT_ADDRESS.parse::<Pubkey>().unwrap(),
    )]
    pub rr3_mint: Account<'info, Mint>,

    // Existing stake record account (must exist)
    #[account(
        mut,
        seeds = [b"stake", user_rr3_token_bag_authority.key().as_ref()],
        bump,
    )]
    pub stake_record: Account<'info, StakeRecord>,

    pub system_program: Program<'info, System>,
}

// New context for claiming RR3 rewards
#[derive(Accounts)]
#[instruction(stake_index: u32, _program_rr3_bag_bump: u8)]
pub struct ClaimRR3Rewards<'info> {
    pub token_program: Program<'info, Token>,

    #[account(mut)]
    pub user_rr3_token_bag: Account<'info, TokenAccount>,

    pub user_authority: Signer<'info>,

    // Program's RR3 token bag (treasury for rewards)
    #[account(
        mut,
        seeds = [b"token_bag"],
        bump = _program_rr3_bag_bump,
    )]
    pub program_rr3_token_bag: Account<'info, TokenAccount>,

    // User's stake record with index
    #[account(
        mut,
        seeds = [b"stake", user_authority.key().as_ref(), stake_index.to_le_bytes().as_ref()],
        bump,
    )]
    pub stake_record: Account<'info, StakeRecord>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(program_rr3_bag_bump: u8, stake_index: u32)]
pub struct UnStake<'info> {
    pub token_program: Program<'info, Token>,

    #[account(mut)]
    pub user_rr3_token_bag: Account<'info, TokenAccount>,

    pub user_rr3_token_bag_authority: Signer<'info>,

    // Staking bag to return principal + rewards from
    #[account(
        mut,
        seeds = [ rr3_mint.key().as_ref() ],
        bump = program_rr3_bag_bump,
    )]
    pub program_rr3_token_bag: Account<'info, TokenAccount>,

    #[account(
        address = RR3_MINT_ADDRESS.parse::<Pubkey>().unwrap(),
    )]
    pub rr3_mint: Account<'info, Mint>,

    // User's stake record with index
    #[account(
        mut,
        seeds = [b"stake", user_rr3_token_bag_authority.key().as_ref(), stake_index.to_le_bytes().as_ref()],
        bump,
        close = user_rr3_token_bag_authority, // Close the account and refund rent
    )]
    pub stake_record: Account<'info, StakeRecord>,

    // User stake counter to update when unstaking
    #[account(
        mut,
        seeds = [b"user_counter", user_rr3_token_bag_authority.key().as_ref()],
        bump,
    )]
    pub user_stake_counter: Account<'info, UserStakeCounter>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct CreateRR3TokenBag<'info> {
    // 1. PDA (so pubkey) for the soon-to-be created RR3 token bag for our program.
    #[account(
        init,
        payer = payer,

        // We use the token mint as a seed for the mapping -> think "HashMap[seeds+bump] = pda"
        seeds = [ RR3_MINT_ADDRESS.parse::<Pubkey>().unwrap().as_ref() ],
        bump,

        // Token Program wants to know what kind of token this token bag is for
        token::mint = rr3_mint,

        // It's a PDA so the authority is itself!
        token::authority = program_rr3_token_bag,
    )]
    pub program_rr3_token_bag: Account<'info, TokenAccount>,

    // 2. The mint 🌈🛤️ because it's needed from above ⬆️ token::mint = ...
    #[account(
        address = RR3_MINT_ADDRESS.parse::<Pubkey>().unwrap(),
    )]
    pub rr3_mint: Account<'info, Mint>,

    // 3. The rent payer
    #[account(mut)]
    pub payer: Signer<'info>,

    // 4. Needed from Anchor for the creation of an Associated Token Account
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
pub struct CreateBuybackTreasury<'info> {
    #[account(
        init,
        payer = payer,
        seeds = [b"buyback_treasury"],
        bump,
        token::mint = rr3_mint,
        token::authority = buyback_treasury_bag,
    )]
    pub buyback_treasury_bag: Account<'info, TokenAccount>,

    #[account(
        address = RR3_MINT_ADDRESS.parse::<Pubkey>().unwrap(),
    )]
    pub rr3_mint: Account<'info, Mint>,

    #[account(mut)]
    pub payer: Signer<'info>,

    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
pub struct CreateTreasuryBag<'info> {
    #[account(
        init,
        payer = payer,
        seeds = [b"token_bag"],
        bump,
        token::mint = rr3_mint,
        token::authority = treasury_bag,
    )]
    pub treasury_bag: Account<'info, TokenAccount>,

    #[account(
        address = RR3_MINT_ADDRESS.parse::<Pubkey>().unwrap(),
    )]
    pub rr3_mint: Account<'info, Mint>,

    #[account(mut)]
    pub payer: Signer<'info>,

    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub rent: Sysvar<'info, Rent>,
}

// Get lock period info context
#[derive(Accounts)]
pub struct GetLockPeriodInfo<'info> {
    /// CHECK: This function is read-only and doesn't require any accounts
    pub signer: Signer<'info>,
}

// Check eligibility context - updated to match stake record PDA pattern
#[derive(Accounts)]
#[instruction(stake_index: u32)]
pub struct CheckEligibility<'info> {
    #[account(
        seeds = [b"stake", staker.key().as_ref(), stake_index.to_le_bytes().as_ref()],
        bump,
    )]
    pub stake_record: Account<'info, StakeRecord>,

    /// CHECK: The staker's public key for PDA derivation
    pub staker: AccountInfo<'info>,
}

#[error_code]
pub enum StakeError {
    #[msg("No rewards available to claim")]
    NoRewardsAvailable,
    #[msg("Insufficient funds in treasury to pay rewards")]
    InsufficientTreasuryFunds,
    #[msg("Too early for distribution - must wait at least 5 minutes between distributions")]
    TooEarlyForDistribution,
    #[msg("No monthly collection recorded for distribution")]
    NoMonthlyCollection,
    #[msg("No stakers available for distribution")]
    NoStakersForDistribution,
    #[msg("Stake has already received rewards for this distribution round")]
    AlreadyReceivedRewardsThisRound,
    #[msg("Professional staking periods ensure eligibility - all stakers are eligible")]
    IneligibleForRewards,
    #[msg("No expense fees available to transfer")]
    NoExpenseFeesToTransfer,
    #[msg("No marketing fees available to transfer")]
    NoMarketingFeesToTransfer,
    #[msg("No fee wallet fees available to transfer")]
    NoFeeWalletFeesToTransfer,
    #[msg("No burn fees available to transfer")]
    NoBurnFeesToTransfer,

    #[msg("Invalid stake index - must match expected next stake number")]
    InvalidStakeIndex,
    #[msg("Invalid lock period - must be 3 months, 6 months, 1 year, 2 years, or 3 years")]
    InvalidLockPeriod,
    #[msg("Tokens are still locked - cannot claim rewards until unlock time")]
    StillLocked,
}
