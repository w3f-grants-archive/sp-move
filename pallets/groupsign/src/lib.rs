// Copyright 2020-2021 Pontem Foundation LTD.
// This file is part of Pontem Network.
// Apache 2.0

//! This pallet enables executing dispatchable calls by using several signers and their signatures.
//! Executed calls have the option to get signers inside by using `T::Origin` as origin from the current pallet.
//! It's useful for some kinds of multisignatures implementations, e.g. Move VM supports multisignature out of the box, 
//! yet it asks for signers of the current transaction.
//! Signers should sign hash `(blake2_256)` generated from data contains encoded: `call`, `valid_since`, `valid_thru`, `caller`, `nonce`.
#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

#[frame_support::pallet]
pub mod pallet {

    use sp_std::prelude::Box;
    use scale_info::TypeInfo;
    use frame_support::{
        dispatch::{Dispatchable, GetDispatchInfo},
        ensure,
        pallet_prelude::*,
    };
    use sp_core::hashing::blake2_256;
    use frame_system::pallet_prelude::*;
    use sp_std::vec::Vec;
    use sp_runtime::{
        traits::{Verify, IdentifyAccount},
        verify_encoded_lazy,
    };

    /// Configure the pallet by specifying the parameters and types on which it depends.
    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// Events.
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

        /// Origin.
        type MyOrigin: From<Origin<Self>> + Into<<Self as frame_system::Config>::Origin>;

        /// Call types.
        type Call: Parameter
            + Dispatchable<Origin = Self::Origin>
            + GetDispatchInfo
            + From<frame_system::Call<Self>>;

        /// Public key type.
        /// Inspiration - https://github.com/JoshOrndorff/recipes/issues/142
        type Public: IdentifyAccount<AccountId = Self::AccountId>
            + Clone
            + TypeInfo
            + Encode
            + Decode
            + PartialEq
            + sp_std::fmt::Debug;

        /// Signature type.
        /// Inspiration - https://github.com/JoshOrndorff/recipes/issues/142
        type Signature: Verify<Signer = Self::Public>
            + Member
            + Decode
            + Encode
            + TypeInfo
            + PartialEq
            + sp_std::fmt::Debug;
    }

    #[pallet::origin]
    #[derive(PartialEq, Eq, Encode, Decode, Clone, TypeInfo)]
    #[cfg_attr(feature = "std", derive(Debug))]
    pub struct Origin<T: Config> {
        signers: Vec<T::AccountId>,
    }

    impl<T: Config> Origin<T> {
        pub fn new(signers: Vec<T::AccountId>) -> Self {
            Self { signers }
        }

        pub fn signers(&self) -> &[T::AccountId] {
            &self.signers
        }
    }

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    #[pallet::event]
    pub enum Event<T: Config> {
        DispatchableExecuted(

        )
    }

    // Errors inform users that something went wrong.
    #[pallet::error]
    pub enum Error<T> {
        // Era validator error (means valid_since, valid_thru don't pass filter).
        EreValidationError,

        // When signatures length doesn't match signers length.
        SignaturesLengthDoesntMatch,

        // Can't verify signature.
        SignatureVerificationError,
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Do groupsign call.
        #[pallet::weight(0)]
        pub fn groupsign_call(
            origin: OriginFor<T>,
            signed_call: Box<<T as Config>::Call>,
            signers: Vec<T::AccountId>,
            signatures: Vec<T::Signature>,
            valid_since: T::BlockNumber,
            valid_thru: T::BlockNumber,
        ) -> DispatchResult {
            let caller = ensure_signed(origin)?;

            // Check signatures length match.
            ensure!(
                signatures.len() == signers.len(),
                Error::<T>::SignaturesLengthDoesntMatch
            );

            // Check era.
            let current_block = frame_system::Pallet::<T>::block_number();

            ensure!(
                current_block >= valid_since && current_block < valid_thru,
                Error::<T>::EreValidationError,
            );

            // Get account nonce.
            let nonce = frame_system::Pallet::<T>::account_nonce(&caller);

            // Generate signature.
            let mut call_preimage = signed_call.encode();
            call_preimage.extend(valid_since.encode());
            call_preimage.extend(valid_thru.encode());
            call_preimage.extend(caller.encode());
            call_preimage.extend(nonce.encode());

            let hash = blake2_256(&call_preimage.as_ref());

            // Verify signature.
            let verified = Iterator::zip(signatures.into_iter(), signers.clone().into_iter())
                .all(|(sig, signer)| verify_encoded_lazy(&sig, &hash, &signer));

            ensure!(verified, Error::<T>::SignatureVerificationError);

            // Do dispatch call.
            let origin = Origin::new(signers.clone());
            let _ = signed_call.dispatch(T::MyOrigin::from(origin).into()); // result

            // TODO: work with weight here.
            // Ok(get_result_weight(result)
            // .map(|actual_weight| {
            //	T::WeightInfo::as_multi_complete(
            //		other_signatories_len as u32,
            //		call_len as u32,
            //	)
            //	.saturating_add(actual_weight)
            //})
            //.into())

            // TODO: emit event.

            Ok(())
        }
    }
}
