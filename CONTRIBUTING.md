# Contributing to nightfall_4_Starknet

Thanks for contributing! 🎉  
This repository is a **fork** of `nightfall_4_CE`, but all development happens **here**.

Please follow the steps below to avoid accidentally opening PRs against the upstream repo.

---

## Important: Where to open Pull Requests

Although this repository is forked from `nightfall_4_CE`, **all pull requests must target this repository**:

**`EYBlockchain/nightfall_4_Starknet`**

Do **NOT** open PRs against `nightfall_4_CE` unless explicitly requested.

---

## How to open a Pull Request (recommended workflow)

### 1. Create a feature branch
```bash
git checkout -b <your-name>/<short-description>
```

### 2. Push your branch to this repo
Make sure your origin remote points to nightfall_4_Starknet.
```bash
git push -u origin HEAD
```

### 3. Open the Pull Request in this repo
Open this link in your browser and select your branch:
```bash
https://github.com/EYBlockchain/nightfall_4_Starknet/compare
```

## PR quality & review process

- **Small, focused PRs (single concern).** Split large work into reviewable chunks. Prefer multiple PRs over one mega-PR.

- **PRs must be self-contained and well described.** Include:
  - a clear title,
  - brief context (why this change is needed),
  - a comprehensive description of what changed and how it was validated (tests, screenshots/logs, etc.).

- **Reviews are collaborative.** Expect a detailed back-and-forth to reach a high-quality outcome.

- **Authors should not resolve review comments themselves.**  
  Comments should be resolved by the **reviewer** once they are satisfied with the fix or clarification. This ensures feedback is validated and closed with reviewer consent.

- **Respond to each review comment with a fixup commit link.**  
  When addressing feedback, reply to each comment with a link to the specific fixup commit that implements the change. This lets reviewers verify updates incrementally.

- **Squash fixups before merge.**  
  After review is complete, squash fixup commits into the main branch history prior to merge (keep the final history clean).

