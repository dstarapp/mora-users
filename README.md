# Mora Users Canister

Mora Users canister is mainly used to record mora's basic user data, which consists of two parts:

- users_index: users index canister, use user principal map to users canister.
- users: users canister, record 1000 users, include base info 、user subscribe index、user planet index.

A users canister is automatically created by users index canister for every 1000 users.

## How to build
```bash
dfx build users
dfx build users_index
```

## How to deploy
```bash
dfx deploy users_index --argument '(principal "dao_canister_principal")'
```
