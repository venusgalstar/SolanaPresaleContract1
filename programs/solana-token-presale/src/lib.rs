use anchor_lang::{prelude::*, solana_program};
use anchor_spl::token::{self, Mint, Token, TokenAccount, Transfer};
use anchor_spl::associated_token::{AssociatedToken};
use std::mem::size_of;
use num_traits::checked_pow;
use solana_program::{program::invoke_signed};


declare_id!("FKVLuN7zhKh4xjVv7264SkW6oGKgPzHa8hz7pqK9KwzL");

pub const GLOBAL_STATE_SEED: &[u8] = b"GLOBAL_STATE_SEED";

pub const USER_STATE_SEED: &[u8] = b"USER_STATE_SEED";

pub const SOL_VAULT_SEED: &[u8] = b"SOL_VAULT_SEED";

#[program]
pub mod solana_token_presale {
    use anchor_lang::solana_program::{program::invoke, system_instruction};

    use super::*;

    pub fn create_global_state(
        _ctx: Context<CreateGlobalState>,
        token_price : u64,
        token_decimal: u64,
    ) -> Result<()> {
        msg!("CreateGlobalState start!!!");
        let global_state = &mut _ctx.accounts.global_state;
        global_state.bump = _ctx.bumps.global_state;
        global_state.mint = _ctx.accounts.mint.key();
        global_state.vault = _ctx.accounts.vault.key();
        global_state.is_initialized = 1;
        global_state.alt_mint = _ctx.accounts.alt_mint.key();
        global_state.alt_vault = _ctx.accounts.alt_vault.key();
        global_state.sol_vault = _ctx.accounts.sol_vault.key();
        global_state.authority = _ctx.accounts.authority.key();
        global_state.token_price = token_price;
        global_state.token_decimal = token_decimal;
        global_state.amount = 0;

        emit!(GlobalStateCreated {
            global_state: _ctx.accounts.global_state.key(),
            mint: _ctx.accounts.mint.key()
        });
        msg!("CreateGlobalState end!!!");
        Ok(())
    }
    pub fn update_global_state(
        _ctx: Context<UpdateGlobalState>,
        token_price : u64,
        token_decimal: u64,
    ) -> Result<()> {
        let global_state = &mut _ctx.accounts.global_state;
        global_state.token_price = token_price;
        global_state.token_decimal = token_decimal;
        Ok(())
    }

    pub fn deposit_token(_ctx: Context<DepositToken>, amount: u64 ) -> Result<()> {
        let cpi_accounts = Transfer {
            from: _ctx.accounts.user_vault.to_account_info(),
            to: _ctx.accounts.pool_vault.to_account_info(),
            authority: _ctx.accounts.authority.to_account_info(),
        };
        let cpi_program = _ctx.accounts.token_program.to_account_info();
        let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);
        token::transfer(cpi_ctx, amount)?;
        Ok(())
    }

    pub fn claim_token(_ctx: Context<DepositToken>, amount: u64) -> Result<()> {
        let admin = _ctx.accounts.global_state.authority;
        let authority = _ctx.accounts.authority.key();
        require_keys_eq!(admin, authority, PreSaleError::NotAllowedAuthority);

        let global_state = &mut _ctx.accounts.global_state;
        let cpi_accounts = Transfer {
            from: _ctx.accounts.pool_vault.to_account_info(),
            to: _ctx.accounts.user_vault.to_account_info(),
            authority: global_state.to_account_info(),
        };

        let seeds = &[GLOBAL_STATE_SEED, &[global_state.bump]];
        let signer = &[&seeds[..]];
        let cpi_program = _ctx.accounts.token_program.to_account_info();
        let cpi_ctx = CpiContext::new_with_signer(cpi_program, cpi_accounts, signer);
        token::transfer(cpi_ctx, amount)?;
        Ok(())
    }

    pub fn buy_token(_ctx: Context<BuyToken>, amount: u64) -> Result<()> {
        msg!("but token start!");
        let global_alt_token = _ctx.accounts.global_state.alt_mint;
        let alt_token = _ctx.accounts.alt_mint.key();
        require_keys_eq!(global_alt_token, alt_token, PreSaleError::InvalidToken);

        let accts = _ctx.accounts;
        accts.user_state.authority = accts.user.key();
        invoke(
            &system_instruction::transfer(&accts.user.key(), &accts.sol_vault.key(), amount),
            &[
                accts.user.to_account_info().clone(),
                accts.sol_vault.clone(),
                accts.system_program.to_account_info().clone(),
            ],
        )?;
        accts.global_state.amount = accts.global_state.amount.checked_add(amount).unwrap();
        let global_state = &mut accts.global_state;
        
        let token_price = global_state.token_price;
        let token_decimal_value = checked_pow(10u64, global_state.token_decimal.try_into().unwrap()).unwrap();
        let token_amount = amount.checked_div(token_price).unwrap().checked_mul(token_decimal_value).unwrap();
        let cpi_accounts = Transfer {
            from: accts.pool_vault.to_account_info(),
            to: accts.user_vault.to_account_info(),
            authority: global_state.to_account_info(),
        };

        let seeds = &[GLOBAL_STATE_SEED, &[global_state.bump]];
        let signer = &[&seeds[..]];
        let cpi_program = accts.token_program.to_account_info();
        let cpi_ctx = CpiContext::new_with_signer(cpi_program, cpi_accounts, signer);
        token::transfer(cpi_ctx, token_amount)?;
        msg!("buy token OK");

        Ok(())
    }

    pub fn swap_token(_ctx: Context<SwapToken>, amount: u64) -> Result<()> {
        msg!("SwapToken start!");
        let global_state = &mut _ctx.accounts.global_state;
        let alt_mint = _ctx.accounts.alt_mint.key();
        let mint = _ctx.accounts.mint.key();
        require_keys_eq!(alt_mint, global_state.alt_mint, PreSaleError::InvalidToken);
        require_keys_eq!(mint, global_state.mint, PreSaleError::InvalidToken);

        let cpi_accounts = Transfer {
            from: _ctx.accounts.user_alt_vault.to_account_info(),
            to: _ctx.accounts.pool_alt_vault.to_account_info(),
            authority: _ctx.accounts.authority.to_account_info(),
        };
        let cpi_program = _ctx.accounts.token_program.to_account_info();
        let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);
        token::transfer(cpi_ctx, amount)?;

        let global_state = &mut _ctx.accounts.global_state;
        let cpi_accounts = Transfer {
            from: _ctx.accounts.pool_vault.to_account_info(),
            to: _ctx.accounts.user_vault.to_account_info(),
            authority: global_state.to_account_info(),
        };
        let seeds = &[GLOBAL_STATE_SEED, &[global_state.bump]];
        let signer = &[&seeds[..]];
        let cpi_program = _ctx.accounts.token_program.to_account_info();
        let cpi_ctx = CpiContext::new_with_signer(cpi_program, cpi_accounts, signer);
        token::transfer(cpi_ctx, amount)?;
        msg!("SwapToken OK");
        Ok(())
    }

    pub fn claim_sol(_ctx: Context<ClaimSol>, amount: u64) -> Result<()> {
        let admin = _ctx.accounts.global_state.authority;
        let user = _ctx.accounts.user.key();
        require_keys_eq!(admin, user, PreSaleError::NotAllowedAuthority);
        let accts = _ctx.accounts;
        let bump = _ctx.bumps.sol_vault;
        invoke_signed(
            &system_instruction::transfer(&accts.sol_vault.key(), &accts.user.key(), amount),
            &[
                accts.sol_vault.to_account_info().clone(),
                accts.user.to_account_info().clone(),
                accts.system_program.to_account_info().clone(),
            ],
            &[&[SOL_VAULT_SEED, &[bump]]],
        )?;
        accts.global_state.amount = accts.global_state.amount.checked_sub(amount).unwrap();
        Ok(())
    }
}


#[derive(Accounts)]
pub struct SwapToken<'info> {
    #[account(
        seeds = [GLOBAL_STATE_SEED], 
        bump,
    )]
    pub global_state: Account<'info, GlobalState>,

    #[account(mut)]
    pub authority: Signer<'info>,

    #[account(mut, constraint = pool_alt_vault.owner == global_state.key())]
    pub pool_alt_vault: Box<Account<'info, TokenAccount>>,

    pub alt_mint: Box<Account<'info, Mint>>,
    #[account(
        init_if_needed, 
        constraint = user_alt_vault.owner == authority.key(), 
        associated_token::mint=alt_mint,
        associated_token::authority=authority,
        payer=authority,
    )]
    pub user_alt_vault: Box<Account<'info, TokenAccount>>,

    #[account(mut, constraint = pool_vault.owner == global_state.key())]
    pub pool_vault: Box<Account<'info, TokenAccount>>,

    pub mint: Box<Account<'info, Mint>>,
    #[account(
        init_if_needed, 
        constraint = user_vault.owner == authority.key(), 
        associated_token::mint=mint,
        associated_token::authority=authority,
        payer=authority,
    )]
    pub user_vault: Box<Account<'info, TokenAccount>>,

    pub system_program: Program<'info, System>,
    #[account(constraint = token_program.key == &token::ID)]
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
}

#[derive(Accounts)]
pub struct BuyToken<'info> {
    #[account(
        mut,
        seeds = [GLOBAL_STATE_SEED], 
        bump
    )]
    pub global_state: Account<'info, GlobalState>,

    #[account(mut)]
    pub user: Signer<'info>,

    #[account(
        mut,
        seeds = [SOL_VAULT_SEED],
        bump
    )]
    /// CHECK: this should be set by admin
    pub sol_vault: AccountInfo<'info>,
    
    #[account(
        init_if_needed,
        seeds = [global_state.key().as_ref(), user.key().as_ref()],
        bump,
        payer = user,
        space = 8 + size_of::<UserAccount>(),
    )]
    pub user_state: Account<'info, UserAccount>,

    #[account(mut, constraint = pool_vault.owner == global_state.key())]
    pub pool_vault: Box<Account<'info, TokenAccount>>,

    pub alt_mint: Box<Account<'info, Mint>>,
    
    #[account(
        init_if_needed, 
        constraint = user_vault.owner == user.key(), 
        associated_token::mint=alt_mint,
        associated_token::authority=user,
        payer=user,
    )]
    pub user_vault: Account<'info, TokenAccount>,

    pub system_program: Program<'info, System>,
    #[account(constraint = token_program.key == &token::ID)]
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
}


#[derive(Accounts)]
pub struct DepositToken<'info> {
    #[account(
        seeds = [GLOBAL_STATE_SEED],
        bump,
    )]
    pub global_state: Account<'info, GlobalState>,

    #[account(mut)]
    pub authority: Signer<'info>,

    #[account(mut, constraint = pool_vault.owner == global_state.key())]
    pub pool_vault: Box<Account<'info, TokenAccount>>,
    #[account(mut, constraint = user_vault.owner == authority.key())]
    pub user_vault: Box<Account<'info, TokenAccount>>,

    pub system_program: Program<'info, System>,
    #[account(constraint = token_program.key == &token::ID)]
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
}

#[derive(Accounts)]
pub struct ClaimSol<'info> {
    #[account(
        mut,
        seeds = [GLOBAL_STATE_SEED],
        bump,
    )]
    pub global_state: Account<'info, GlobalState>,

    #[account(mut)]
    pub user: Signer<'info>,

    #[account(
        mut,
        seeds = [SOL_VAULT_SEED],
        bump
    )]
    /// CHECK: this should be checked with address in global_state
    pub sol_vault: AccountInfo<'info>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct UpdateGlobalState<'info> {
    #[account(
        mut,
        seeds = [GLOBAL_STATE_SEED],
        bump,
    )]
    pub global_state: Account<'info, GlobalState>,

    #[account(mut)]
    pub authority: Signer<'info>,
}

#[derive(Accounts)]
pub struct CreateGlobalState<'info> {
    #[account(
        init,
        seeds = [GLOBAL_STATE_SEED],
        bump,
        payer = authority,
        space = 8 + size_of::<GlobalState>()
    )]
    pub global_state: Account<'info, GlobalState>,

    /// CHECK: this should be set by admin
    pub sol_vault: AccountInfo<'info>,

    pub mint: Box<Account<'info, Mint>>,
    #[account(
        init,
        associated_token::mint=mint,
        associated_token::authority=global_state,
        payer = authority,
    )]
    pub vault: Account<'info, TokenAccount>,

    pub alt_mint: Box<Account<'info, Mint>>,
    #[account(
        init,
        associated_token::mint=alt_mint,
        associated_token::authority=global_state,
        payer = authority,
    )]
    pub alt_vault: Account<'info, TokenAccount>,
    
    #[account(mut)]
    pub authority: Signer<'info>,
    pub system_program: Program<'info, System>,
    #[account(constraint = token_program.key == &token::ID)]
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
}

impl<'info> CreateGlobalState<'info> {
    pub fn validate(&self) -> Result<()> {
        if self.global_state.is_initialized == 1 {
            require!(
                self.global_state.authority.eq(&self.authority.key()),
                PreSaleError::NotAllowedAuthority
            )
        }
        Ok(())
    }
}

#[account]
#[derive(Default)]
pub struct UserAccount {
    pub bump: u8,
    pub global_state: Pubkey,
    pub authority: Pubkey,
    pub amount: u64,
}

#[account]
#[derive(Default)]
pub struct GlobalState {
    pub bump: u8,
    pub authority: Pubkey,
    pub amount: u64,
    pub sol_vault: Pubkey,
    pub alt_mint: Pubkey,
    pub alt_vault: Pubkey,
    pub mint: Pubkey,
    pub vault: Pubkey,
    pub is_initialized: u8,
    pub token_price: u64,
    pub token_decimal: u64,
}

#[event]
pub struct GlobalStateCreated {
    global_state: Pubkey,
    mint: Pubkey,
}

#[event]
pub struct UserCreated {
    global_state: Pubkey,
    user: Pubkey,
    authority: Pubkey,
}
#[error_code]
pub enum PreSaleError {
    #[msg("Not allowed authority")]
    NotAllowedAuthority,

    #[msg("Should be specied tokens")]
    InvalidToken,
}