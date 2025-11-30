# Contributing to Sudan Digital Archive API

Welcome! We're thrilled that you're interested in contributing to the Sudan Digital Archive API. This is a community-driven open source project dedicated to preserving and making accessible the collective memory and experiences of the Sudanese people. **Your help is genuinely welcome and valued.**

## Our Values

The Sudan Digital Archive operates as a collective guided by three core principles:

1. **Mutual support and understanding** — We support one another through collaboration and demonstrate empathy in all interactions
2. **Clear ethical standards** — We maintain professional standards and transparent decision-making processes
3. **Cultural resonance and community connection** — We remain guided by and connected to Sudanese perspectives and values

## Getting Started

### Before You Start

**We encourage you to reach out before beginning work.** Whether you've found an issue to tackle or have a new idea, opening a discussion first helps us:

- Avoid duplicate efforts
- Ensure your work aligns with the project's direction
- Provide guidance and support
- Build community around the work

**Don't worry about timing.** We're all volunteers, so responses may take some time, but we're committed to helping you every step of the way.

## How to Contribute

### 1. Find or Create an Issue

- **Looking for work?** Check our [Issues](https://github.com/Sudan-Digital-Archive/sudan-digital-archive-api/issues) page for tasks labeled `good first issue` or `help wanted`
- **Have an idea?** [Open a new issue](https://github.com/Sudan-Digital-Archive/sudan-digital-archive-api/issues/new) to discuss it first. Tell us what you're thinking—we'd love to hear from you!
- **Found a bug?** [Create an issue](https://github.com/Sudan-Digital-Archive/sudan-digital-archive-api/issues/new) with details about what happened

### 2. Engage in Discussion

Comment on the issue and tell us you're interested in working on it. This helps us:
- Understand your background and experience level
- Provide resources or guidance specific to your needs
- Ensure alignment with our project goals
- Build community around the work

We'll do our best to respond promptly, though responses may take a few days.

### 3. Set Up Your Development Environment

```bash
# Clone the repository
git clone https://github.com/Sudan-Digital-Archive/sudan-digital-archive-api.git
cd sudan-digital-archive-api

# Follow the setup instructions in README.MD
```

### 4. Make Your Changes

- Create a descriptive branch name: `feature/add-api-endpoint` or `fix/resolve-database-connection`
- Follow the coding standards and style conventions used in the project
- Write clear, descriptive commit messages
- Include tests for new functionality
- Update documentation as needed

**Before pushing, run these checks locally:**

```bash
# Set up your test environment variable
export JWT_SECRET="test-secret-for-local-development"

# Format your code
cargo fmt

# Check for linting issues (must pass with no warnings)
cargo clippy -- -D warnings

# Run tests
cargo test --verbose
```

All of these checks run in CI/CD, so running them locally first will help catch any issues before you push!

### 5. Submit a Pull Request

- Link your PR to the relevant issue(s)
- Provide a clear description of your changes and why you made them
- Be patient—reviews may take time, but we'll provide thoughtful feedback
- Respond to feedback constructively; we're all learning together

## Code of Conduct

We're committed to providing a welcoming and inspiring community for all. Please review our [Code of Conduct](https://sudandigitalarchive.com/code-of-conduct) before participating. We take our ethical standards seriously and expect all contributors to do the same.

## Contribution Areas

While code contributions are wonderful, there are many ways to contribute:

- **Bug reports** — Help us identify issues
- **Documentation** — Improve clarity and completeness
- **Design & UX** — Suggest improvements to the user experience
- **Testing** — Help us find edge cases and ensure quality
- **Community** — Help answer questions and welcome new contributors
- **Translation** — Help make the project accessible in multiple languages
- **Ideas** — Share your thoughts on features and improvements

## Questions or Need Help?

- **GitHub Issues** — The best place to ask questions is on the [issue](https://github.com/Sudan-Digital-Archive/sudan-digital-archive-api/issues) you're working on
- **Contact Us** — Visit https://sudandigitalarchive.com/contact-us for other communication channels

## Recognition

We value and recognize all contributions. Contributors will be acknowledged in project documentation and releases. Whether you contribute code, documentation, ideas, or support to other community members, your work matters.

## Project Structure

Quick overview to help you navigate:

- `/src` — Main application code (Rust)
- `/entity` — Database entities and models
- `/migration` — Database migrations
- `/src/routes` — API endpoints
- `/src/services` — Business logic
- `/src/repos` — Data access layer
- `README.MD` — Project overview and setup

## Technical Guidelines

- **Language:** Rust
- **Format code:** Run `cargo fmt` before committing
- **Linting:** Address any warnings from `cargo clippy`
- **Testing:** Ensure all tests pass with `cargo test`
- **Database:** Follow migration patterns established in `/migration`

## Making Your First Contribution

Starting with your first open source contribution can feel intimidating, but we're here to support you! If you're new to open source:

1. Look for issues labeled `good first issue`
2. Open the issue and introduce yourself
3. Ask questions—there are no silly questions in our community
4. We'll be happy to provide guidance and support

Remember: you don't need to be an expert to contribute. We value all experience levels and are committed to helping you grow.

## Thank You

The Sudan Digital Archive exists because of people like you who believe in the power of preserving and sharing Sudanese stories and experiences. Thank you for being part of this mission.

---

For more information about our project, visit:
- [Sudan Digital Archive](https://sudandigitalarchive.com)
- [Our Mission](https://sudandigitalarchive.com/mission)
- [Who We Are](https://sudandigitalarchive.com/who-are-we)
