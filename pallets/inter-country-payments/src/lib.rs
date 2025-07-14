#![cfg_attr(not(feature = "std"), no_std)]

//! # Inter-Country Payments Pallet
//! 
//! This pallet implements direct government-to-government payment functionality for VedGov:
//! - Verified government-only transactions
//! - Multi-signature authorization requirements
//! - Transparent bilateral payment settlement
//! - Real-time cross-border transfers
//! - Compliance and audit trail

use frame_support::{
    codec::{Decode, Encode},
    dispatch::{DispatchError, DispatchResult},
    traits::{Currency, Get, ReservableCurrency},
    PalletId, RuntimeDebug,
};
use frame_system::ensure_signed;
use scale_info::TypeInfo;
use sp_runtime::{
    traits::{AccountIdConversion, Saturating, Zero},
    Perbill,
};
use sp_std::{vec::Vec, collections::btree_map::BTreeMap};

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::*;

    const STORAGE_VERSION: StorageVersion = StorageVersion::new(1);

    #[pallet::pallet]
    #[pallet::generate_store(pub(super) trait Store)]
    #[pallet::storage_version(STORAGE_VERSION)]
    pub struct Pallet<T>(_);

    #[pallet::config]
    pub trait Config: frame_system::Config {
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
        
        /// The currency used for government payments (VGV tokens)
        type Currency: Currency<Self::AccountId> + ReservableCurrency<Self::AccountId>;

        /// Maximum length for country codes (ISO 3166-1 alpha-3)
        #[pallet::constant]
        type MaxCountryCodeLength: Get<u32>;

        /// Maximum length for payment references
        #[pallet::constant]
        type MaxReferenceLength: Get<u32>;

        /// Fixed fee for government transactions
        #[pallet::constant]
        type GovernmentTransactionFee: Get<u128>;

        /// The pallet id for sovereign account derivation
        #[pallet::constant]
        type PalletId: Get<PalletId>;
    }

    /// Government verification status
    #[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo)]
    pub enum GovernmentStatus {
        /// Government is verified and active
        Active,
        /// Government is verified but temporarily suspended
        Suspended,
        /// Government verification is pending
        Pending,
        /// Government access has been revoked
        Revoked,
    }

    /// Payment purpose categories for government transactions
    #[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo)]
    pub enum PaymentPurpose {
        /// Bilateral trade settlement
        TradeSettlement {
            trade_agreement_id: Vec<u8>,
            goods_reference: Vec<u8>,
        },
        /// Development aid and assistance
        DevelopmentAid {
            program_id: Vec<u8>,
            beneficiary_country: Vec<u8>,
        },
        /// Emergency assistance (disaster relief, humanitarian aid)
        EmergencyAssistance {
            disaster_reference: Vec<u8>,
            urgency_level: UrgencyLevel,
        },
        /// Diplomatic expenses (embassy operations, diplomatic missions)
        DiplomaticExpenses {
            embassy_code: Vec<u8>,
            expense_category: ExpenseCategory,
        },
        /// International organization contributions
        InternationalContribution {
            organization: Vec<u8>,
            contribution_type: ContributionType,
        },
        /// Loan repayments between governments
        LoanRepayment {
            loan_agreement_id: Vec<u8>,
            installment_number: u32,
        },
    }

    /// Urgency levels for emergency assistance
    #[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo)]
    pub enum UrgencyLevel {
        Low,
        Medium,
        High,
        Critical,
    }

    /// Expense categories for diplomatic payments
    #[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo)]
    pub enum ExpenseCategory {
        EmbassyOperations,
        ConsularServices,
        DiplomaticMissions,
        CulturalExchange,
        SecurityExpenses,
    }

    /// Types of international contributions
    #[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo)]
    pub enum ContributionType {
        MembershipFees,
        SpecialAssessment,
        VoluntaryContribution,
        PeacekeepingFunds,
    }

    /// Government registration information
    #[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo)]
    pub struct GovernmentInfo {
        /// ISO 3166-1 alpha-3 country code
        pub country_code: Vec<u8>,
        /// Official government name
        pub government_name: Vec<u8>,
        /// Government type (Treasury, Central Bank, Ministry of Finance, etc.)
        pub institution_type: InstitutionType,
        /// Verification status
        pub status: GovernmentStatus,
        /// Multi-signature threshold for transactions
        pub signature_threshold: u32,
        /// Authorized signatories
        pub authorized_signatories: Vec<T::AccountId>,
        /// Registration timestamp
        pub registered_at: u64,
    }

    /// Types of government institutions
    #[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo)]
    pub enum InstitutionType {
        Treasury,
        CentralBank,
        MinistryOfFinance,
        MonetaryAuthority,
        FinancialIntelligenceUnit,
        CustomsAuthority,
    }

    /// Inter-country payment transaction
    #[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo)]
    pub struct InterCountryPayment<AccountId> {
        /// Payment unique identifier
        pub payment_id: u64,
        /// Sending government account
        pub from_government: AccountId,
        /// Receiving government account
        pub to_government: AccountId,
        /// Payment amount in VGV tokens
        pub amount: u128,
        /// Purpose of the payment
        pub purpose: PaymentPurpose,
        /// Reference number for tracking
        pub reference: Vec<u8>,
        /// Required signatures
        pub required_signatures: Vec<AccountId>,
        /// Collected signatures
        pub signatures: Vec<(AccountId, Vec<u8>)>, // (signer, signature)
        /// Transaction timestamp
        pub timestamp: u64,
        /// Payment status
        pub status: PaymentStatus,
    }

    /// Payment transaction status
    #[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo)]
    pub enum PaymentStatus {
        /// Payment initiated, awaiting signatures
        Pending,
        /// All required signatures collected, ready for execution
        Authorized,
        /// Payment successfully executed
        Completed,
        /// Payment failed or was rejected
        Failed,
        /// Payment was cancelled before execution
        Cancelled,
    }

    #[pallet::storage]
    #[pallet::getter(fn government_info)]
    /// Government registration information
    pub type GovernmentRegistry<T: Config> = 
        StorageMap<_, Blake2_128Concat, T::AccountId, GovernmentInfo>;

    #[pallet::storage]
    #[pallet::getter(fn country_to_account)]
    /// Mapping from country code to government account
    pub type CountryToAccount<T: Config> = 
        StorageMap<_, Blake2_128Concat, Vec<u8>, T::AccountId>;

    #[pallet::storage]
    #[pallet::getter(fn payment_info)]
    /// Inter-country payment transactions
    pub type PaymentRegistry<T: Config> = 
        StorageMap<_, Blake2_128Concat, u64, InterCountryPayment<T::AccountId>>;

    #[pallet::storage]
    #[pallet::getter(fn next_payment_id)]
    /// Next available payment ID
    pub type NextPaymentId<T: Config> = StorageValue<_, u64, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn total_payments_volume)]
    /// Total volume of payments processed
    pub type TotalPaymentsVolume<T: Config> = StorageValue<_, u128, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn government_payment_history)]
    /// Payment history for each government
    pub type GovernmentPaymentHistory<T: Config> = 
        StorageMap<_, Blake2_128Concat, T::AccountId, Vec<u64>>;

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// Government was registered [account, country_code, institution_type]
        GovernmentRegistered {
            account: T::AccountId,
            country_code: Vec<u8>,
            institution_type: InstitutionType,
        },

        /// Payment was initiated [payment_id, from, to, amount]
        PaymentInitiated {
            payment_id: u64,
            from_government: T::AccountId,
            to_government: T::AccountId,
            amount: u128,
        },

        /// Payment signature was added [payment_id, signer]
        PaymentSignatureAdded {
            payment_id: u64,
            signer: T::AccountId,
        },

        /// Payment was authorized (all signatures collected) [payment_id]
        PaymentAuthorized {
            payment_id: u64,
        },

        /// Payment was completed [payment_id, from, to, amount]
        PaymentCompleted {
            payment_id: u64,
            from_government: T::AccountId,
            to_government: T::AccountId,
            amount: u128,
        },

        /// Payment was cancelled [payment_id, reason]
        PaymentCancelled {
            payment_id: u64,
            reason: Vec<u8>,
        },

        /// Government status was updated [account, new_status]
        GovernmentStatusUpdated {
            account: T::AccountId,
            status: GovernmentStatus,
        },
    }

    #[pallet::error]
    pub enum Error<T> {
        /// Government already registered
        GovernmentAlreadyRegistered,
        /// Government not found or not registered
        GovernmentNotFound,
        /// Government is not active (suspended/revoked/pending)
        GovernmentNotActive,
        /// Payment not found
        PaymentNotFound,
        /// Payment already completed or cancelled
        PaymentNotPending,
        /// Insufficient balance for payment
        InsufficientBalance,
        /// Not authorized to sign for this government
        NotAuthorizedSigner,
        /// Signature already provided by this signer
        SignatureAlreadyProvided,
        /// Payment amount must be greater than zero
        ZeroAmount,
        /// Country code too long
        CountryCodeTooLong,
        /// Reference too long
        ReferenceTooLong,
        /// Cannot pay to the same government
        SelfPayment,
        /// Signature threshold must be greater than zero
        InvalidSignatureThreshold,
        /// Payment already has all required signatures
        PaymentAlreadyAuthorized,
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Register a government entity
        #[pallet::weight(10_000)]
        #[pallet::call_index(0)]
        pub fn register_government(
            origin: OriginFor<T>,
            account: T::AccountId,
            country_code: Vec<u8>,
            government_name: Vec<u8>,
            institution_type: InstitutionType,
            signature_threshold: u32,
            authorized_signatories: Vec<T::AccountId>,
        ) -> DispatchResult {
            ensure_root(origin)?; // Only sudo can register governments initially

            ensure!(
                country_code.len() <= T::MaxCountryCodeLength::get() as usize,
                Error::<T>::CountryCodeTooLong
            );

            ensure!(
                signature_threshold > 0,
                Error::<T>::InvalidSignatureThreshold
            );

            ensure!(
                !GovernmentRegistry::<T>::contains_key(&account),
                Error::<T>::GovernmentAlreadyRegistered
            );

            let gov_info = GovernmentInfo {
                country_code: country_code.clone(),
                government_name,
                institution_type: institution_type.clone(),
                status: GovernmentStatus::Active,
                signature_threshold,
                authorized_signatories,
                registered_at: Self::current_timestamp(),
            };

            GovernmentRegistry::<T>::insert(&account, &gov_info);
            CountryToAccount::<T>::insert(&country_code, &account);

            Self::deposit_event(Event::GovernmentRegistered {
                account,
                country_code,
                institution_type,
            });

            Ok(())
        }

        /// Initiate an inter-country payment
        #[pallet::weight(10_000)]
        #[pallet::call_index(1)]
        pub fn initiate_payment(
            origin: OriginFor<T>,
            to_government: T::AccountId,
            amount: u128,
            purpose: PaymentPurpose,
            reference: Vec<u8>,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;

            ensure!(amount > 0, Error::<T>::ZeroAmount);
            ensure!(who != to_government, Error::<T>::SelfPayment);
            ensure!(
                reference.len() <= T::MaxReferenceLength::get() as usize,
                Error::<T>::ReferenceTooLong
            );

            // Verify both governments are registered and active
            let from_gov = Self::government_info(&who).ok_or(Error::<T>::GovernmentNotFound)?;
            let to_gov = Self::government_info(&to_government).ok_or(Error::<T>::GovernmentNotFound)?;

            ensure!(from_gov.status == GovernmentStatus::Active, Error::<T>::GovernmentNotActive);
            ensure!(to_gov.status == GovernmentStatus::Active, Error::<T>::GovernmentNotActive);

            // Check balance including fees
            let total_cost = amount.saturating_add(T::GovernmentTransactionFee::get());
            let balance = T::Currency::free_balance(&who);
            ensure!(
                balance >= total_cost.saturated_into(),
                Error::<T>::InsufficientBalance
            );

            // Reserve the payment amount + fees
            T::Currency::reserve(&who, total_cost.saturated_into())?;

            let payment_id = Self::next_payment_id();
            let payment = InterCountryPayment {
                payment_id,
                from_government: who.clone(),
                to_government: to_government.clone(),
                amount,
                purpose,
                reference,
                required_signatures: from_gov.authorized_signatories.clone(),
                signatures: Vec::new(),
                timestamp: Self::current_timestamp(),
                status: PaymentStatus::Pending,
            };

            PaymentRegistry::<T>::insert(payment_id, &payment);
            NextPaymentId::<T>::put(payment_id.saturating_add(1));

            // Add to payment history
            let mut history = Self::government_payment_history(&who).unwrap_or_default();
            history.push(payment_id);
            GovernmentPaymentHistory::<T>::insert(&who, history);

            Self::deposit_event(Event::PaymentInitiated {
                payment_id,
                from_government: who,
                to_government,
                amount,
            });

            Ok(())
        }

        /// Add signature to a payment
        #[pallet::weight(10_000)]
        #[pallet::call_index(2)]
        pub fn sign_payment(
            origin: OriginFor<T>,
            payment_id: u64,
            signature: Vec<u8>,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;

            let mut payment = Self::payment_info(payment_id).ok_or(Error::<T>::PaymentNotFound)?;
            ensure!(payment.status == PaymentStatus::Pending, Error::<T>::PaymentNotPending);

            // Verify signer is authorized
            ensure!(
                payment.required_signatures.contains(&who),
                Error::<T>::NotAuthorizedSigner
            );

            // Check if signature already provided
            ensure!(
                !payment.signatures.iter().any(|(signer, _)| signer == &who),
                Error::<T>::SignatureAlreadyProvided
            );

            // Add signature
            payment.signatures.push((who.clone(), signature));

            // Check if we have all required signatures
            let gov_info = Self::government_info(&payment.from_government)
                .ok_or(Error::<T>::GovernmentNotFound)?;

            if payment.signatures.len() >= gov_info.signature_threshold as usize {
                payment.status = PaymentStatus::Authorized;
                Self::deposit_event(Event::PaymentAuthorized { payment_id });
            }

            PaymentRegistry::<T>::insert(payment_id, &payment);

            Self::deposit_event(Event::PaymentSignatureAdded {
                payment_id,
                signer: who,
            });

            Ok(())
        }

        /// Execute an authorized payment
        #[pallet::weight(10_000)]
        #[pallet::call_index(3)]
        pub fn execute_payment(origin: OriginFor<T>, payment_id: u64) -> DispatchResult {
            let _who = ensure_signed(origin)?;

            let mut payment = Self::payment_info(payment_id).ok_or(Error::<T>::PaymentNotFound)?;
            ensure!(payment.status == PaymentStatus::Authorized, Error::<T>::PaymentNotPending);

            // Transfer the payment amount
            T::Currency::repatriate_reserved(
                &payment.from_government,
                &payment.to_government,
                payment.amount.saturated_into(),
                frame_support::traits::BalanceStatus::Free,
            )?;

            // Pay transaction fee (unreserve and burn)
            let fee_amount = T::GovernmentTransactionFee::get().saturated_into();
            T::Currency::unreserve(&payment.from_government, fee_amount);
            T::Currency::withdraw(
                &payment.from_government,
                fee_amount,
                frame_support::traits::WithdrawReasons::FEE,
                frame_support::traits::ExistenceRequirement::AllowDeath,
            )?;

            payment.status = PaymentStatus::Completed;
            PaymentRegistry::<T>::insert(payment_id, &payment);

            // Update total volume
            TotalPaymentsVolume::<T>::put(
                Self::total_payments_volume().saturating_add(payment.amount)
            );

            Self::deposit_event(Event::PaymentCompleted {
                payment_id,
                from_government: payment.from_government,
                to_government: payment.to_government,
                amount: payment.amount,
            });

            Ok(())
        }

        /// Cancel a pending payment
        #[pallet::weight(10_000)]
        #[pallet::call_index(4)]
        pub fn cancel_payment(
            origin: OriginFor<T>,
            payment_id: u64,
            reason: Vec<u8>,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;

            let mut payment = Self::payment_info(payment_id).ok_or(Error::<T>::PaymentNotFound)?;
            ensure!(payment.status == PaymentStatus::Pending, Error::<T>::PaymentNotPending);

            // Only the initiating government can cancel
            ensure!(payment.from_government == who, Error::<T>::NotAuthorizedSigner);

            // Unreserve the funds
            let total_reserved = payment.amount.saturating_add(T::GovernmentTransactionFee::get());
            T::Currency::unreserve(&who, total_reserved.saturated_into());

            payment.status = PaymentStatus::Cancelled;
            PaymentRegistry::<T>::insert(payment_id, &payment);

            Self::deposit_event(Event::PaymentCancelled {
                payment_id,
                reason,
            });

            Ok(())
        }

        /// Update government status (admin only)
        #[pallet::weight(10_000)]
        #[pallet::call_index(5)]
        pub fn update_government_status(
            origin: OriginFor<T>,
            government: T::AccountId,
            new_status: GovernmentStatus,
        ) -> DispatchResult {
            ensure_root(origin)?;

            let mut gov_info = Self::government_info(&government)
                .ok_or(Error::<T>::GovernmentNotFound)?;

            gov_info.status = new_status.clone();
            GovernmentRegistry::<T>::insert(&government, &gov_info);

            Self::deposit_event(Event::GovernmentStatusUpdated {
                account: government,
                status: new_status,
            });

            Ok(())
        }
    }

    impl<T: Config> Pallet<T> {
        /// Get current timestamp (simplified for demo)
        fn current_timestamp() -> u64 {
            // In a real implementation, this would get the actual block timestamp
            0u64
        }

        /// Get payment statistics for a government
        pub fn get_government_stats(account: &T::AccountId) -> Option<(u32, u128, u128)> {
            let history = Self::government_payment_history(account)?;
            let payment_count = history.len() as u32;
            
            let (total_sent, total_received) = history.iter()
                .filter_map(|&id| Self::payment_info(id))
                .fold((0u128, 0u128), |(sent, received), payment| {
                    if payment.status == PaymentStatus::Completed {
                        if payment.from_government == *account {
                            (sent.saturating_add(payment.amount), received)
                        } else if payment.to_government == *account {
                            (sent, received.saturating_add(payment.amount))
                        } else {
                            (sent, received)
                        }
                    } else {
                        (sent, received)
                    }
                });

            Some((payment_count, total_sent, total_received))
        }
    }
}

// Runtime API for government payment queries
sp_api::decl_runtime_apis! {
    pub trait InterCountryPaymentsApi<AccountId> {
        fn get_government_info(account: AccountId) -> Option<GovernmentInfo>;
        fn get_payment_info(payment_id: u64) -> Option<InterCountryPayment<AccountId>>;
        fn get_government_stats(account: AccountId) -> Option<(u32, u128, u128)>;
        fn get_total_payments_volume() -> u128;
    }
}
