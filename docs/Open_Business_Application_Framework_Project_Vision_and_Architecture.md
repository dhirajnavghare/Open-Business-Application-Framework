# Open Business Application Framework
## Project Vision, Product Requirements & Architecture Direction

**Document type:** Master project vision and architecture baseline  
**Status:** Initial design reference — not an implementation specification  
**Version:** 1.0  
**Date:** 15 November 2026  
**Working title:** Open Business Application Framework (descriptive placeholder; final project name not selected)  
**License:** To be selected before public release

---

## 1. Executive Summary

The Open Business Application Framework is an open-source, API-first, configuration-driven business application framework built primarily in Rust.

Its purpose is to provide a reusable foundation for creating, operating, and extending multiple independent business applications without repeatedly implementing common backend capabilities.

The framework should provide reusable platform capabilities and business engines—such as identity, tenancy, permissions, customers, catalog, inventory, orders, billing, payments, scheduling, workflows, notifications, documents, reporting, and integrations—while allowing each business domain to retain its own specialized data models and rules.

The framework is not intended to be one universal business application with every feature enabled. It is intended to be a foundation from which different business applications can be assembled using reusable modules, domain extensions, configuration, and independent client applications.

Examples of applications that could eventually use the framework include:

- Retail and grocery management
- Pharmacy management
- Restaurant and point-of-sale systems
- Hotel and hospitality management
- Gym and membership management
- Salon and appointment management
- Education and coaching management
- Rental and property management
- Repair and field-service management
- Manufacturing and distribution management
- Logistics and fleet management
- Professional services and invoicing

These are potential application templates, not commitments to implement every application in the first release.

### 1.1 Core proposition

**Build the shared foundation once. Assemble different business applications from reusable capabilities. Keep each application independently configurable and deployable.**

### 1.2 Product principles

1. Open-source first, with contribution and extension practices designed into the repository.
2. API-first and frontend-independent.
3. Rust-based implementation using established Rust infrastructure libraries.
4. Modular, composable Rust framework with a pluggable application runtime.
5. Configuration-driven application composition.
6. Reusable business engines with domain-specific extensions.
7. Strong multi-application and multi-tenant isolation.
8. Database portability where it can be tested and supported reliably.
9. Multiple deployment compositions without premature distributed-system complexity.
10. Stable public contracts for applications and community extensions.
11. Security, data integrity, observability, and upgrade safety as core requirements.
12. Incremental delivery, validated through real business workflows.

---

## 2. Vision, Goals and Non-Goals

### 2.1 Vision

Create a dependable open-source Rust framework that makes it practical to build and operate multiple independent business applications on a common, reusable backend foundation.

### 2.2 Primary goals

- Provide a consistent application runtime.
- Provide reusable platform and business capabilities.
- Allow applications to enable and configure modules.
- Support multiple applications and organizations with strict isolation.
- Expose consistent, versioned APIs.
- Keep frontends independent from backend implementation.
- Support both simple single-business deployments and larger multi-tenant deployments.
- Allow developers to add first-party and community extensions.
- Provide predictable configuration, migration, upgrade, and compatibility processes.
- Keep the initial deployment and contribution model understandable.

### 2.3 Non-goals for the initial releases

The project should not initially attempt to:

- Replace Rust's HTTP, async, database-driver, serialization, or cryptographic ecosystems.
- Implement a general-purpose ORM or database engine.
- Build every listed business application before validating the framework.
- Start with a microservices architecture.
- Provide arbitrary, unrestricted runtime loading of native Rust libraries.
- Make every business rule dynamically configurable.
- Generate complete frontend applications automatically.
- Require Redis, Kubernetes, a message broker, or a cloud provider for basic operation.
- Guarantee identical behavior across database engines without compatibility tests.
- Implement country-specific regulatory behavior in the universal core.
- Promise a fixed number of tenants, users, or transactions before performance testing.

---

## 3. Architecture Direction

### 3.1 Architectural principle

Build a **modular, composable Rust framework with a pluggable application runtime**.

The repository should be a Cargo workspace containing independently organized crates. Platform capabilities, reusable business engines, and domain-specific extensions should have explicit boundaries and stable contracts. Applications should be assembled from the modules they need, with the runtime determining how those modules are hosted and executed.

A crate or module boundary is a code, package, and ownership boundary. It does not automatically imply a separate process, service, database, or deployment. Conversely, modules should not be tightly coupled merely because they may run in the same process.

This approach supports open-source collaboration, independent development, testing, and future evolution while allowing operators to choose a deployment composition appropriate to their needs.

### 3.2 Architecture style — Modular, Composable Rust Framework with a Pluggable Application Runtime

Build the framework as a **multi-crate Rust workspace with a composable application runtime**. Do not make a unified runtime or microservices the mandatory architecture of the project.

The runtime should support multiple deployment compositions:

- **Unified runtime:** multiple modules execute in one process. This is the simplest option for development, smaller deployments, and workflows that benefit from in-process coordination.
- **Separated processes:** API servers, background workers, schedulers, and selected workloads can run as separate processes while using the same module contracts and application definition.
- **Independent services:** selected modules may be hosted as separately deployed services when real requirements justify independent scaling, fault isolation, security boundaries, release cadence, or operational ownership.

These are deployment choices, not different definitions of a module. The framework should avoid forcing every application into a distributed architecture, while preserving the option to separate selected workloads later.

Each module must have well-defined:

- Responsibilities and scope
- Dependencies and dependency direction
- Public interfaces
- Data ownership
- Configuration schema
- Permissions
- Events and event contracts
- API routes, where applicable
- Database migrations
- Background jobs, where applicable
- Extension points
- Compatibility requirements
- Tests and documentation

The framework architecture should support:

- A unified runtime for applications that prefer a simple deployment.
- Separately runnable API, worker, scheduler, and other workload processes.
- Independent development, testing, and explicit dependency management for modules.
- Clear boundaries between platform capabilities, reusable business engines, and domain extensions.
- Optional module registration and activation through application configuration.
- Community-developed extensions without requiring changes to the framework core.
- Versioned public interfaces and compatibility policies.
- Selective service deployment where justified by measurable requirements.
- Multiple application compositions without requiring every application to enable every module.

Do not introduce distributed infrastructure by default. Choose a unified runtime when it is sufficient; separate processes or services only when operational, scaling, security, or organizational requirements justify the additional complexity.

### 3.3 Dependency direction

Prefer dependencies that point inward toward stable domain and framework contracts.

- Domain logic should not depend directly on HTTP handlers.
- Business modules should not depend on frontend-specific behavior.
- Core contracts should not depend on a specific database engine.
- Infrastructure adapters should implement interfaces required by the application and domain layers.
- Modules should not access another module's private tables or internal implementation.
- Cross-module operations should use explicit service interfaces, domain events, or application-level orchestration.

Avoid circular crate dependencies. If two modules need to communicate, introduce a stable contract or move shared concepts into an appropriately scoped crate.

### 3.4 Architectural layers

1. **Application definitions:** application identity, enabled modules, configuration, domain extensions, and presets.
2. **Framework runtime:** application lifecycle, module registry, configuration resolution, request context, and orchestration.
3. **Platform capabilities:** identity, tenancy, authorization, audit, events, jobs, and integrations.
4. **Reusable business engines:** customers, catalog, inventory, orders, pricing, billing, scheduling, and other reusable capabilities.
5. **Domain extensions:** specialized behavior for restaurant, pharmacy, hotel, retail, and other domains.
6. **Infrastructure adapters:** HTTP, database, cache, object storage, queues, email, and external providers.
7. **Deployment processes:** API server, workers, database, and optional supporting infrastructure.

### 3.5 Module independence and deployment composition

**Module independence must not depend on deployment independence.** A module should be independently understandable, testable, configurable, and governed by explicit contracts even when it runs in the same process as other modules.

The framework should aim for the following properties:

| Property | Architectural direction |
|---|---|
| Separate source/package organization | Required where it creates a meaningful boundary |
| Clear public interfaces | Required |
| Independent testing | Required |
| Explicit dependencies | Required |
| Independent configuration | Required |
| Data ownership | Explicit and enforced |
| Independent versioning | Introduce where useful and practical |
| Independent process | Optional |
| Independent deployment | Optional |
| Independent database | Optional; determined by domain and operational needs |

Modules must communicate through published contracts, application services, or events—not by reaching into another module's private implementation or tables. In-process calls may implement these contracts in a unified runtime; remote adapters may implement them when modules are separated.

### 3.6 Architecture evolution

The architecture should preserve the option to separate selected modules, but should not be designed around hypothetical microservices.

Before moving a module to a separate process or service, verify:

- It has a clear business boundary.
- Its data ownership is understood.
- Its interfaces are stable.
- Its operational scaling need is demonstrated.
- Cross-module transactions and consistency requirements are understood.
- Deployment, monitoring, security, and recovery costs are justified.

---

## 4. Technology Strategy

### 4.1 Language

Use **Rust** as the primary implementation language.

Reasons:

- Memory safety without a garbage collector.
- Strong type system and compile-time checks.
- High performance and efficient resource use.
- Suitable for concurrent API and worker workloads.
- Good fit for long-running services and container deployments.
- Strong foundation for explicit module contracts.

Rust does not automatically guarantee correct business logic, financial calculations, tenant isolation, or security. These require sound design, code review, and testing.

### 4.2 Recommended initial stack

| Area | Initial direction |
|---|---|
| Language | Rust |
| Async runtime | Tokio |
| HTTP/API framework | Axum |
| Serialization | Serde |
| Primary database | PostgreSQL |
| ORM / database access | Evaluate and initially use SeaORM where suitable |
| API description | OpenAPI, with Utoipa as a candidate |
| Validation | Explicit domain validation; Validator and JSON Schema where appropriate |
| Logging and tracing | Tracing |
| Error handling | Thiserror and Anyhow |
| Configuration | Figment or a similarly maintained configuration library |
| Authentication primitives | Established, reviewed Rust libraries |
| Password hashing | Argon2id through a maintained implementation |
| Cache | Optional Redis |
| File storage | Local storage initially; S3-compatible abstraction |
| Deployment | Docker and Docker Compose |
| Testing | Rust unit/integration tests plus API and compatibility test suites |

These are initial technology choices, not permanent constraints. Verify maintenance status, security advisories, feature suitability, and compatibility before adopting dependencies.

### 4.3 Infrastructure to reuse

Use established libraries for:

- HTTP protocol handling
- Async execution
- Database drivers and connection pools
- Serialization and deserialization
- Cryptographic primitives
- Password hashing
- Logging and tracing
- API documentation generation
- Configuration parsing
- File and network I/O

Do not implement these infrastructure foundations from scratch.

### 4.4 What the project should own

The project's distinctive value should be in its own:

- Application runtime and lifecycle
- Module contracts and registry
- Configuration resolution and lifecycle
- Application and tenant management
- Authorization model and policy integration
- Business engines
- Domain extension contracts
- Workflow and rules abstractions
- Event and automation conventions
- API conventions and metadata
- Preset format
- CLI and developer experience
- Upgrade and compatibility policies

### 4.5 Database support policy

PostgreSQL should be the primary production reference implementation.

MySQL/MariaDB and SQLite may be supported through tested compatibility layers, but support must be demonstrated by automated tests. Do not assume that changing a connection URL makes the application portable.

Database differences to test include:

- Data types and precision
- Constraints and indexes
- Migration syntax
- JSON behavior
- Transaction and locking semantics
- Upsert behavior
- Pagination and query semantics
- Generated identifiers
- Collation and case sensitivity
- Concurrent write behavior

SQLite is appropriate for development, testing, embedded, or lightweight single-instance scenarios where its concurrency and operational characteristics are acceptable. It should not be presented as the default database for a horizontally scaled multi-instance deployment.

Avoid database-specific SQL in domain modules. Where specialized SQL is necessary, isolate it behind infrastructure adapters and test it for each supported engine.

---

## 5. Application and Tenant Model

### 5.1 Conceptual hierarchy

The platform should distinguish the following concepts:

- **Platform:** one running installation or managed platform environment.
- **Application:** a business application definition, such as Pharmacy or Restaurant POS.
- **Organization:** a tenant or business using an application.
- **Branch / Location:** an operating unit within an organization.
- **User:** an identity that may have access to one or more organizations.
- **Membership:** the relationship between a user and an organization, including roles and scope.
- **Device / Client:** a registered client or integration acting on behalf of an authorized user or service.

A possible hierarchy is:

```text
Platform
└── Application
    ├── Organization A
    │   ├── Branch A1
    │   └── Branch A2
    └── Organization B
        └── Branch B1
```

The exact relationship between applications and organizations must be finalized during detailed domain design. In particular, decide whether one organization may subscribe to multiple applications and how shared data between those applications is represented.

### 5.2 Tenant isolation

Tenant isolation is a mandatory security boundary.

Requirements:

- Establish tenant context from trusted authentication and authorization context.
- Never trust an unchecked tenant identifier supplied by a client.
- Enforce tenant scope in service and data-access layers.
- Include tenant ownership in relevant database constraints and indexes.
- Ensure background jobs and events carry validated tenant context.
- Prevent cross-tenant access through search, reports, exports, files, and webhooks.
- Test negative cases, not only successful access.
- Record security-relevant access and administrative changes.

### 5.3 Deployment and isolation modes

Support deployment choices progressively:

1. **Shared deployment and shared database:** multiple organizations share infrastructure with strict tenant isolation.
2. **Shared application deployment with dedicated tenant database:** for tenants needing stronger data separation.
3. **Dedicated deployment:** isolated runtime and infrastructure for customers with specific operational, regulatory, or contractual requirements.

Database-per-tenant and dedicated deployments add substantial provisioning, migration, connection, monitoring, backup, and support complexity. They should be introduced when justified.

### 5.4 Tenant-aware routing

If multiple databases are introduced, use a controlled tenant-to-database routing abstraction.

- Resolve tenant placement from trusted platform metadata.
- Establish routing before starting tenant-scoped work.
- Keep connection pools bounded.
- Avoid opening an unbounded pool per tenant.
- Ensure migrations can be orchestrated across tenant databases.
- Track placement, health, version, and migration state.
- Plan tenant movement and recovery procedures.

---

## 6. Configuration-Driven Application Framework

### 6.1 Purpose

Applications should be assembled from reusable modules and validated configuration rather than requiring a new backend implementation for each business type.

Configuration should control supported variations. It must not replace strongly typed domain logic where correctness and integrity require explicit code.

### 6.2 Configuration categories

An application definition may contain:

- Application identity and version
- Business preset
- Enabled modules and module versions
- Module dependencies
- Application-level settings
- Organization-level settings
- Branch-level overrides
- Custom fields and validation schemas
- Roles and permissions
- Workflow definitions
- Tax and billing settings
- Currency and localization
- Numbering rules
- Document templates
- Notification rules
- Integrations and webhooks
- Feature entitlements and usage limits

### 6.3 Configuration inheritance

A possible precedence model is:

```text
Platform defaults
    ↓
Application defaults
    ↓
Organization configuration
    ↓
Branch configuration
    ↓
Authorized runtime overrides, if explicitly supported
```

The final precedence and override rules must be documented and deterministic.

### 6.4 Configuration lifecycle

Support:

- Draft
- Validate
- Preview / inspect differences
- Publish
- Activate
- Roll back
- Archive

Also provide:

- Version history
- Actor and timestamp for each change
- Schema validation
- Module dependency validation
- Permission checks
- Compatibility checks
- Import and export
- Environment-specific configuration
- Controlled cache invalidation

Critical historical transactions must not silently change when configuration changes. Store appropriate snapshots of values such as applied tax rates, prices, currency, invoice terms, and document-template versions.

### 6.5 Presets

Presets are versioned starting templates, not rigid application types.

A preset should define:

- Preset identifier and version
- Intended business domain
- Suggested modules
- Module dependencies
- Default configuration
- Default roles and permissions
- Optional workflows
- Required domain extensions
- Compatibility range

Users should be able to customize a preset without losing the ability to understand its origin or upgrade path.

### 6.6 Custom fields and schemas

Support metadata-driven fields for controlled extension of business records.

Requirements:

- Field type and validation
- Required/optional status
- Default values where safe
- Visibility and edit permissions
- Search and filtering policy
- Versioning
- Data migration behavior
- API schema exposure

Do not use untyped JSON blobs as a substitute for core relational models. Decide carefully which custom fields belong in JSON storage, typed extension tables, or dedicated domain entities.

---

## 7. Module and Extension Architecture

### 7.1 Module categories

Organize modules into:

1. **Platform modules:** identity, tenancy, permissions, configuration, audit.
2. **Shared business modules:** customers, suppliers, catalog, inventory, orders, pricing.
3. **Financial modules:** billing, taxes, payments, expenses, ledger, reconciliation.
4. **Operational modules:** scheduling, resources, workforce, workflow, notifications.
5. **Domain extensions:** restaurant, pharmacy, hotel, retail, manufacturing, and others.
6. **Integration modules:** payment providers, messaging providers, accounting systems, external APIs.

### 7.2 Module metadata

Each module should declare, as applicable:

- Stable module ID
- Display name and description
- Semantic version
- Framework compatibility range
- Dependencies and optional dependencies
- Configuration schema
- Permissions
- Events published and consumed
- Routes and API capabilities
- Background jobs
- Database migrations
- Extension points
- Capability declarations
- Tests and documentation

### 7.3 Module lifecycle

Define lifecycle operations such as:

- Discover
- Validate
- Register
- Initialize
- Enable for an application
- Configure
- Start
- Stop
- Disable
- Upgrade
- Migrate
- Uninstall, where safe

Disabling or uninstalling a module must not silently delete business data. Data retention, migration, and removal behavior must be explicit.

### 7.4 Rust implementation strategy

Begin with statically compiled Rust modules registered by the host application.

This is the recommended initial approach because it is easier to build, test, review, distribute, and secure than arbitrary runtime loading of native libraries.

Distinguish:

- **Build-time availability:** the module is included in the compiled binary.
- **Runtime activation:** the module is enabled for a particular application or organization.

Later extension options may include:

- External services integrated through versioned APIs and webhooks.
- Sandboxed plugin runtimes for carefully selected use cases.
- A stable cross-language extension protocol.

Do not promise a stable native Rust plugin ABI unless a deliberate compatibility and security design is completed.

### 7.5 Community extension model

Community modules should be able to:

- Live in independent repositories or workspace packages.
- Depend on published framework contracts.
- Declare module metadata and dependencies.
- Register supported services, routes, events, jobs, and permissions.
- Provide migrations and tests.
- Document supported framework versions.
- Pass contract and security checks.

The framework should publish module templates, examples, contribution guidance, and compatibility policies.

---

## 8. Reusable Business Engines

The following are candidate reusable capabilities. Implement them incrementally and keep their responsibilities bounded.

### 8.1 Platform and identity

- User and service identity
- Authentication and session management
- Organizations and memberships
- Roles and permissions
- API credentials and scopes
- Device registration
- Tenant context
- Audit records

### 8.2 Customer and supplier management

- Customer profiles
- Supplier profiles
- Contact details
- Addresses
- Customer groups
- Tags and segments
- Interaction history
- Consent and communication preferences where applicable

### 8.3 Catalog and pricing

- Products and services
- Categories and attributes
- Variants and units
- Price lists
- Cost and selling prices
- Tiered pricing
- Promotions and discounts
- Coupons and offer rules
- Tax classification references

### 8.4 Inventory and warehouse

- Warehouses and storage locations
- Stock balances
- Stock movements
- Receipts and issues
- Transfers
- Adjustments
- Reservations
- Reorder rules
- Batch and serial tracking as optional capabilities
- Expiry tracking where relevant
- Stock valuation policies

### 8.5 Orders and transactions

- Quotations
- Sales orders
- Purchase orders
- Order lines and modifiers
- Status transitions
- Returns and cancellations
- Fulfilment
- Idempotent transaction creation
- Transaction history

### 8.6 Scheduling and booking

- Resources and availability
- Staff schedules
- Appointment slots
- Reservations
- Capacity rules
- Cancellations
- Waitlists
- Recurrence where appropriate
- Time zones and daylight-saving behavior

### 8.7 Membership and subscription

- Membership plans
- Subscription lifecycle
- Renewals
- Pauses and cancellations
- Entitlements
- Usage tracking
- Recurring billing integration

### 8.8 Workflow and approvals

- State transitions
- Approval policies
- Multi-step approvals
- Delegation
- Escalation
- Thresholds
- Timeouts
- Human tasks
- Workflow history

### 8.9 Billing, tax, and financial operations

- Invoices and credit notes
- Tax calculation
- Discounts
- Partial and advance payments
- Split payments
- Refunds
- Receivables and payables where in scope
- Expenses
- Settlements
- Cash reconciliation
- Financial reporting
- Document numbering

Financial requirements:

- Use decimal arithmetic with explicit precision and rounding rules.
- Never use binary floating point for monetary calculations.
- Store currency explicitly.
- Preserve historical snapshots of applied tax and pricing.
- Use database transactions for related financial operations.
- Make payment and posting operations idempotent.
- Maintain immutable or append-only audit history for finalized financial records.
- Correct finalized records through appropriate reversal, adjustment, or credit mechanisms rather than destructive edits.
- Keep country-specific tax and compliance behavior in jurisdiction-specific modules.
- Do not hardcode one country's tax rules in the universal core.

The platform's own subscription billing, if introduced, must remain distinct from the business financial engine used by applications.

### 8.10 Workforce and resource management

- Employees and teams
- Roles and assignments
- Shifts and availability
- Skills and certifications where relevant
- Resource allocation
- Equipment and asset availability
- Time and attendance integrations

### 8.11 Notifications and automation

- Email, SMS, push, and in-app notification adapters
- Templates
- User preferences
- Scheduled notifications
- Event-triggered automation
- Retry and delivery history
- Provider abstraction

### 8.12 Documents and reporting

- Document metadata
- File storage abstraction
- Templates
- PDF generation
- Receipts and print layouts
- Export jobs
- Report definitions
- Operational dashboards
- Audit and activity exports

### 8.13 Integration and reconciliation

- API credentials
- Webhooks
- External provider adapters
- Import/export pipelines
- Payment reconciliation
- Inventory reconciliation
- External reference mapping
- Integration health and delivery history

---

## 9. Domain Extensions

The framework must not force every industry into one universal entity model.

Shared capabilities should be reused, while domain-specific concepts and rules remain in domain extensions.

Examples:

### 9.1 Restaurant extension

Potential specialized concepts:

- Tables and table layouts
- Kitchen tickets
- Order courses
- Modifiers and add-ons
- Recipes and ingredients
- Preparation stations
- Service charges
- Split bills
- Dine-in, takeaway, and delivery workflows

### 9.2 Pharmacy extension

Potential specialized concepts:

- Drug and product classifications
- Batch and expiry management
- Prescription references and workflows
- Supplier and purchase traceability
- Controlled inventory policies
- Regulatory reporting integrations, where applicable

### 9.3 Hotel extension

Potential specialized concepts:

- Room types and rooms
- Availability and occupancy
- Reservations
- Check-in and check-out
- Housekeeping
- Room charges
- Guest services

### 9.4 Retail extension

Potential specialized concepts:

- POS terminals
- Promotions
- Barcode workflows
- Product variants
- Returns and exchanges
- Store-specific stock operations

### 9.5 Additional potential domains

- Grocery and supermarket
- Gym and fitness
- Salon and spa
- Laundry and dry-cleaning
- Repair and service center
- Coaching and training
- Clinic and appointment management
- Equipment and vehicle rental
- Property and housing management
- Manufacturing and job work
- Courier and fleet operations
- Agriculture and dairy operations
- Professional services
- Event and venue management

These domains should be added only when the shared framework and domain boundaries have been validated.

---

## 10. API-First Design

### 10.1 API principles

- The backend must not require a particular frontend.
- Use versioned REST APIs initially.
- Publish OpenAPI specifications.
- Use consistent request, response, and error formats.
- Validate input at the API boundary and enforce business rules in domain services.
- Provide pagination, filtering, sorting, and field-selection policies where appropriate.
- Support idempotency for operations that may be retried.
- Apply rate limits and request-size limits.
- Define backward-compatibility and deprecation policies.
- Keep API authorization independent of frontend visibility.

### 10.2 API areas

Potential API namespaces include:

- Authentication and sessions
- Applications and application configuration
- Organizations, branches, and memberships
- Users, roles, and permissions
- Modules, presets, and schemas
- Customers and suppliers
- Catalog and pricing
- Inventory and warehouses
- Orders and transactions
- Bookings and subscriptions
- Workflows and approvals
- Invoices, payments, and expenses
- Resources and workforce
- Documents and notifications
- Reports and exports
- Integrations and webhooks
- Audit logs and platform operations

Exact endpoints should be designed in a separate API specification.

### 10.3 Metadata APIs

Provide APIs that allow independent client applications and future administration tools to discover:

- Application capabilities
- Enabled modules
- Configuration schemas
- Field definitions
- Validation rules
- Navigation metadata, where appropriate
- Permission-aware actions
- Workflow states
- Reference data and dropdown options

Metadata may guide client rendering, but the backend remains authoritative for validation, authorization, and business rules.

### 10.4 API versioning

Define:

- Versioning strategy
- Compatibility guarantees
- Deprecation period
- Error taxonomy
- Pagination conventions
- Idempotency semantics
- Authentication and scope conventions
- Webhook event versioning

Avoid breaking third-party consumers without a documented migration path.

---

## 11. Security and Authorization

Security is a platform requirement, not an optional module.

### 11.1 Authentication

- Secure password hashing using Argon2id or an appropriately reviewed alternative.
- Secure session handling or carefully designed token authentication.
- Refresh-token rotation and revocation where applicable.
- Multi-factor authentication as a future or optional capability.
- Account recovery and credential lifecycle.
- Service accounts and API credentials.
- Secure secret storage and rotation.

### 11.2 Authorization

Support a layered authorization model:

- Role-based access control (RBAC)
- Organization and branch scope
- Resource or record-level policies where needed
- Field-level visibility and edit rules where justified
- Approval thresholds
- API credential scopes
- Service-to-service permissions
- Delegation and temporary access where required

Attribute-based access control (ABAC) may be introduced for cases where roles alone are insufficient. Avoid building a complex policy language before concrete use cases require it.

### 11.3 Security controls

- Strict tenant isolation
- Input validation
- Parameterized database queries
- Secure file access
- Request-size and rate limits
- Audit logging
- TLS in transit
- Encryption at rest where appropriate
- Secret redaction in logs
- Dependency and vulnerability scanning
- Secure defaults
- CORS policy
- CSRF protection where cookie-based browser authentication is used
- Protection against replay and duplicate operations
- Regular backup and restore tests
- Security review of extension points

Do not store payment-card data unless a specific compliant architecture and business need justify it. Prefer certified payment providers and tokenized payment references.

---

## 12. Events, Jobs and Automation

### 12.1 Event model

Use explicit, versioned domain event contracts.

Potential event names:

- `customer.created`
- `order.created`
- `order.updated`
- `order.cancelled`
- `inventory.stock.received`
- `inventory.stock.adjusted`
- `inventory.stock.low`
- `inventory.batch.expiring`
- `booking.created`
- `booking.cancelled`
- `subscription.renewed`
- `invoice.created`
- `payment.received`
- `payment.failed`
- `workflow.status.changed`
- `document.generated`

Events must have stable identifiers, timestamps, tenant/application context where applicable, schema versions, and trace/correlation identifiers.

### 12.2 Reliable event delivery

Use a transactional outbox pattern for events that must be published reliably with database changes.

Define:

- Event persistence
- Delivery attempts
- Retry policy
- Exponential backoff
- Dead-letter handling
- Consumer idempotency
- Event retention
- Replay policy
- Delivery observability

Do not claim exactly-once delivery across distributed systems. Design for at-least-once delivery and idempotent consumers.

### 12.3 Background jobs

Use workers for:

- Notifications
- Webhook delivery
- PDF generation
- Bulk import/export
- Scheduled renewals
- Recurring tasks
- Large reports
- Data synchronization
- Reconciliation
- Long-running workflows

Workers must support bounded concurrency, tenant-aware context, retries, cancellation where safe, idempotency, and failure visibility.

The basic platform should not require Redis. Select a durable job/queue strategy appropriate to the initial deployment and introduce additional infrastructure when operational needs justify it.

---

## 13. Scalability and Performance

### 13.1 Scaling strategy

Design for horizontal scaling from the beginning, but deploy only the infrastructure currently required.

#### Stage 1 — Unified or lightly separated deployment

- One application runtime hosting the required modules, or one API process with a worker process when useful
- One PostgreSQL database
- Local or S3-compatible file storage
- Optional cache
- No requirement to deploy each module independently

#### Stage 2 — Horizontal workload scaling

- Load balancer where multiple API instances are needed
- Multiple stateless API instances or runtime instances
- Independently scaled workers where workload patterns require it
- Shared persistent state
- Centralized configuration and secrets
- Shared observability

#### Stage 3 — Workload separation

- Independently scaled workers
- Durable event and job processing
- Optional Redis
- Object storage
- Read replicas where justified
- Reporting workload isolation

#### Stage 4 — Large deployments

- Tenant-aware database placement
- Partitioning of high-volume tables
- Dedicated tenant databases or deployments
- Regional deployment strategies
- Analytics infrastructure
- Selective service extraction where justified

### 13.2 API scalability

API instances should be stateless wherever practical.

Do not rely on:

- In-memory sessions tied to one instance
- Local container storage for durable files
- A single API process for scheduled jobs
- In-memory queues for durable work
- Local-only caches for correctness

Use shared or persistent infrastructure for state that must survive restarts and instance replacement.

### 13.3 Database scalability

Prioritize:

- Correct indexes
- Tenant-aware query patterns
- Connection pooling
- Query optimization
- Cursor pagination for large datasets
- Short transactions
- Bounded query results
- Avoidance of N+1 queries
- Slow-query monitoring
- Read replicas where appropriate
- Partitioning only when measurements justify it

Database sharding should be a later decision based on measured limits and a clear tenant/data placement model.

### 13.4 Resource fairness

One tenant must not be able to consume all shared resources.

Define configurable limits for:

- API request rate
- Concurrent requests
- Worker concurrency
- Job submissions
- Storage usage
- File size
- Export size
- API credentials
- Webhook delivery
- Report execution

Support fair scheduling and workload isolation where necessary.

### 13.5 Performance verification

Measure:

- API latency (p50, p95, p99)
- Throughput
- Error rates
- Database query latency
- Connection pool saturation
- Lock contention
- Queue depth and job duration
- Resource consumption per tenant
- Storage growth
- Recovery time
- Infrastructure cost per active organization

Do not publish capacity claims until repeatable benchmarks have been performed on documented hardware and workload profiles.

---

## 14. Data Portability, Search and Synchronization

### 14.1 Import and export

Provide:

- CSV, JSON, and spreadsheet support where appropriate
- Import preview and validation
- Duplicate detection
- Conflict handling
- Background processing for large jobs
- Progress and error reports
- Organization data export
- Attachment export
- Audit history export

### 14.2 Search

Begin with database-supported search and indexes. Add a dedicated search engine only when scale, relevance, or workload requirements justify it.

Search must respect tenant and permission boundaries.

### 14.3 Offline synchronization

Treat offline support as an optional capability, not a universal requirement.

Potential capabilities:

- Registered devices
- Local operation queues
- Incremental synchronization
- Change cursors or sequence numbers
- Conflict detection and resolution
- Idempotency keys
- Deleted-record synchronization
- Device revocation
- Sync status and error reporting

Offline behavior must be explicitly designed per domain. Financial, inventory, and booking operations may require different conflict and consistency policies.

---

## 15. Localization and Regulatory Adaptation

Support:

- Multiple languages
- Organization time zones
- Locale-aware dates and numbers
- Multiple currencies
- Explicit currency precision and rounding
- Exchange-rate sources where required
- Units of measurement
- Address and phone formats
- Fiscal-year settings
- Country-specific tax modules
- Regional document formats

Country-specific compliance should be implemented as jurisdiction-specific extensions or integrations.

For example, India-specific GST, e-invoicing, and e-way bill integrations should not be hardcoded into the universal financial core. Their legal and technical requirements must be reviewed against current official guidance before implementation.

---

## 16. Developer Experience and Open-Source Governance

### 16.1 Repository strategy

Use a Cargo workspace monorepo initially. It makes coordinated changes, integration tests, examples, and shared versioning easier while the architecture is evolving.

A possible repository structure:

```text
project/
├── Cargo.toml
├── Cargo.lock
├── README.md
├── LICENSE
├── CONTRIBUTING.md
├── CODE_OF_CONDUCT.md
├── SECURITY.md
├── CHANGELOG.md
├── .github/
│   └── workflows/
│
├── crates/
│   ├── core/
│   ├── runtime/
│   ├── module-api/
│   ├── module-registry/
│   ├── configuration/
│   ├── identity/
│   ├── tenancy/
│   ├── permissions/
│   ├── database/
│   ├── api/
│   ├── events/
│   ├── jobs/
│   ├── audit/
│   ├── customers/
│   ├── catalog/
│   ├── inventory/
│   ├── orders/
│   ├── pricing/
│   ├── billing/
│   ├── payments/
│   └── workflow/
│
├── extensions/
│   ├── restaurant/
│   ├── pharmacy/
│   ├── retail/
│   └── examples/
│
├── binaries/
│   ├── api-server/
│   └── worker/
│
├── presets/
├── migrations/
├── examples/
├── tests/
│   ├── unit/
│   ├── integration/
│   ├── api/
│   ├── tenancy/
│   ├── database-compatibility/
│   ├── migrations/
│   └── performance/
│
├── deployment/
│   ├── docker/
│   ├── compose/
│   └── scripts/
│
└── docs/
    ├── architecture/
    ├── modules/
    ├── api/
    ├── operations/
    └── contributing/
```

This is a target direction, not a requirement to create every crate at the beginning. Create crates when a real boundary and implementation exist.

### 16.2 Developer tooling

Consider a CLI for common development and operational tasks:

```bash
framework init
framework server start
framework worker start
framework app list
framework app validate
framework module list
framework config validate
framework config export
framework migrate
framework doctor
```

The command names are illustrative and should be finalized after choosing the project identity and CLI conventions.

### 16.3 Community requirements

Provide:

- Clear contribution guidelines
- Code of conduct
- Security vulnerability reporting process
- License and dependency policy
- Architecture decision records (ADRs)
- Module development guide
- Example extensions
- Issue and pull-request templates
- Automated CI
- Formatting and linting checks
- Release and deprecation policy
- Compatibility matrix
- Maintainer responsibilities

### 16.4 Licensing

Select an open-source license deliberately before public release.

Consider project goals such as:

- Permissive reuse by commercial and non-commercial projects
- Patent provisions
- Compatibility with dependencies
- Community contribution expectations
- Whether a separate contributor license agreement is necessary

Do not assume that calling a project open source automatically grants rights to use every name, logo, dependency, or third-party asset. Review licenses and trademark policies separately.

---

## 17. Administration and Future User Interfaces

The backend must remain usable without a bundled frontend.

A future administration application may provide:

- Application registration
- Organization and branch management
- User and role administration
- Module activation
- Preset selection
- Configuration editing
- Workflow configuration
- Integration setup
- API credential management
- Usage and entitlement views
- Audit and operational views

This administration UI must be a separate project that communicates through documented APIs.

Other independent clients may include web, mobile, desktop, and third-party applications.

The backend controls authorization, validation, business rules, and data integrity. Clients control presentation and interaction.

---

## 18. Deployment and Operations

### 18.1 Initial deployment

Provide Docker images and Docker Compose examples for a practical initial deployment.

Possible processes:

- API server
- Worker
- PostgreSQL
- Optional Redis
- Optional reverse proxy
- Optional object storage service for development

Do not bundle the database inside the API container. Persist database and file data outside ephemeral application containers.

### 18.2 Container requirements

- Multi-stage builds
- Minimal runtime image
- Non-root execution
- Environment-based configuration
- Secret-safe startup
- Health and readiness checks
- Graceful shutdown
- Persistent volumes for required state
- Structured logs
- Resource limits
- Migration command or job
- Documented backup and restore procedure

### 18.3 Production operations

Plan for:

- TLS termination
- Centralized logs
- Metrics and tracing
- Alerting
- Database backups
- Restore verification
- Migration monitoring
- Deployment and rollback
- Secret rotation
- Maintenance mode
- Incident runbooks
- Capacity planning
- Recovery objectives

A backup is not considered reliable until restoration has been tested.

---

## 19. Testing and Quality Strategy

Testing must be part of module development, not postponed until the end.

| Test category | Purpose |
|---|---|
| Unit tests | Validate domain rules and calculations |
| Integration tests | Validate module and infrastructure interactions |
| API tests | Validate contracts and error behavior |
| Tenant isolation tests | Prevent cross-tenant access |
| Authorization tests | Prevent unauthorized operations |
| Database compatibility tests | Validate each supported database |
| Migration tests | Validate fresh install and upgrades |
| Concurrency tests | Validate simultaneous writes and transactions |
| Idempotency tests | Validate safe retries |
| Event and job tests | Validate delivery, retries, and failures |
| Load tests | Measure throughput and latency |
| Stress and endurance tests | Find saturation and long-running failures |
| Recovery tests | Validate backup and restore |
| Security tests | Validate common vulnerabilities and controls |
| End-to-end tests | Validate complete business workflows |

### 19.1 Mandatory release gates

At minimum, releases must pass:

- Tenant isolation tests
- Authorization tests
- Financial correctness tests for financial modules
- Database migration tests
- Supported database compatibility tests
- API contract tests
- Dependency and security checks
- Backup and restore verification for production release candidates

---

## 20. Development Roadmap

The roadmap is intentionally incremental. The goal is to validate the framework before expanding its scope.

### Phase 0 — Project definition

- Select final project name after appropriate checks.
- Select license.
- Define contribution and governance policies.
- Record initial architecture decisions.
- Define supported Rust toolchain policy.
- Define initial supported database and deployment target.

### Phase 1 — Technical foundation

- Create Cargo workspace.
- Establish formatting, linting, and CI.
- Implement core error and context conventions.
- Integrate Tokio and Axum.
- Integrate PostgreSQL access and migrations.
- Add configuration loading.
- Add structured logging and tracing.
- Add Docker development deployment.
- Publish health and version endpoints.

### Phase 2 — Platform foundation

- Identity and authentication
- Organizations and memberships
- Tenant context and isolation
- Roles and permissions
- Application registry
- Module metadata and registry
- Configuration schema and validation
- Audit records
- API documentation

### Phase 3 — First reusable business capabilities

- Customers and suppliers
- Catalog
- Pricing
- Inventory
- Orders
- Basic document generation
- Events and background jobs

### Phase 4 — Financial and operational capabilities

- Billing
- Tax abstraction
- Payments integration
- Refunds and reconciliation
- Scheduling
- Resources
- Workflow and approvals
- Notifications

### Phase 5 — First domain validation

Build two contrasting applications:

**Restaurant POS** to validate:

- Catalog and modifiers
- Tables and orders
- Kitchen workflow
- Inventory consumption
- Billing and payments
- Split bills
- Printing and receipts

**Pharmacy management** to validate:

- Specialized product data
- Batch and expiry tracking
- Inventory movements
- Purchase traceability
- Specialized permissions
- Domain-specific workflows

The goal is to prove that the framework can support both without putting their specialized rules into the universal core.

### Phase 6 — Open-source extension readiness

- Publish module contracts
- Add extension templates
- Add example community module
- Publish compatibility policy
- Add module contract tests
- Document module development
- Add CLI foundations

### Phase 7 — Production hardening

- Performance benchmarks
- Security review
- Database compatibility expansion
- Backup and restore automation
- Observability
- Upgrade and rollback procedures
- Load and concurrency tests
- Operational documentation

### Phase 8 — Expansion

Only after validating the core:

- Retail and grocery
- Hotel management
- Gym and membership
- Salon and appointment management
- Rental management
- Education and coaching
- Repair and field service
- Manufacturing and distribution
- Other domain extensions

### Phase 9 — Advanced platform capabilities

When justified by real use cases:

- Offline synchronization
- Advanced rules engine
- Search infrastructure
- SaaS provisioning and billing
- Usage metering and entitlements
- Dedicated tenant deployments
- Database routing and partitioning
- Selective service extraction
- Additional SDKs and integrations

---

## 21. Architecture Rules — Non-Negotiable Principles

1. Build a modular, composable Rust framework with a pluggable application runtime.
2. Use a multi-crate Rust workspace with clear dependency boundaries.
3. Preserve module independence regardless of whether modules share a process or deployment.
4. Do not build an HTTP server, async runtime, ORM, database driver, or cryptographic primitives from scratch.
5. Keep the backend independent of frontend frameworks.
6. Keep domain rules out of HTTP handlers.
7. Do not create a universal data model that erases domain differences.
8. Reuse business capabilities without forcing unrelated domains into the same specialized entities.
9. Enforce tenant isolation in trusted backend layers.
10. Treat configuration as versioned, validated data.
11. Preserve historical transaction correctness when configuration changes.
12. Make module dependencies explicit.
13. Do not let modules access another module's private data structures or tables.
14. Prefer statically compiled Rust extensions initially.
15. Do not require Redis or another optional service for basic operation.
16. Use reliable event delivery patterns and idempotent consumers.
17. Keep financial calculations precise and auditable.
18. Do not silently delete data when disabling modules.
19. Test database portability rather than assuming it.
20. Publish API and module compatibility policies.
21. Do not introduce distributed services without measurable justification.
22. Do not create empty crates for speculative future functionality.
23. Build and validate a small number of real applications before expanding the catalog.
24. Make security, migrations, observability, and recovery part of the architecture from the beginning.
25. Keep documentation and tests alongside public module contracts.

---

## 22. Definition of Success

The framework is moving toward its intended goal when:

- A developer can create a new application definition without creating a new backend codebase.
- Multiple applications can use the same framework runtime.
- Organizations remain isolated from one another.
- Modules can be enabled and configured through supported APIs and configuration.
- Domain-specific extensions can add specialized behavior without modifying core modules.
- Independent frontend applications can consume the APIs.
- The backend can run without any frontend.
- API instances, workers, and other workloads can be scaled independently where the chosen deployment composition requires it.
- Supported database engines pass compatibility tests.
- Configuration changes are versioned, validated, and auditable.
- Module upgrades have documented migration and compatibility behavior.
- Community developers can build extensions using published contracts.
- Production operators can monitor, back up, restore, upgrade, and recover the platform.
- Performance claims are supported by repeatable benchmarks.

---

## 23. Decisions to Finalize Before Implementation

The master vision intentionally leaves several implementation choices open. Resolve them in dedicated technical design documents.

1. Final project name, repository name, crate prefix, and CLI name.
2. Open-source license and governance model.
3. Rust minimum supported version and release policy.
4. Exact module contract and lifecycle API.
5. Database access strategy and supported SeaORM features.
6. PostgreSQL migration and schema ownership strategy.
7. Tenant-to-application relationship and organization membership model.
8. Shared-database tenant isolation strategy.
9. Configuration storage, schema versioning, and inheritance semantics.
10. Module enablement and data retention behavior.
11. Authentication and session strategy.
12. Authorization policy model.
13. Event envelope and transactional outbox design.
14. Durable job processing implementation.
15. API versioning and error conventions.
16. File storage and document generation approach.
17. Deployment and upgrade orchestration.
18. Initial application scope and acceptance criteria.

Each major decision should be recorded as an Architecture Decision Record (ADR), including context, alternatives, decision, consequences, and date.

---

## 24. Recommended First Technical Design Documents

After accepting this master vision, create these documents separately:

1. **System Architecture Specification** — runtime, layers, crate boundaries, dependency direction, and process model.
2. **Module Contract Specification** — metadata, lifecycle, dependencies, permissions, events, migrations, and extension points.
3. **Multi-Tenancy and Authorization Design** — tenant context, memberships, data isolation, and policy enforcement.
4. **Database Architecture** — schema ownership, migration strategy, transaction boundaries, and database compatibility.
5. **Configuration and Preset Specification** — schema, inheritance, lifecycle, validation, versioning, and rollback.
6. **API Standards** — routes, authentication, errors, pagination, idempotency, OpenAPI, and versioning.
7. **Events and Background Jobs Design** — event envelope, outbox, retries, idempotency, workers, and dead-letter handling.
8. **Security Architecture** — threat model, secrets, sessions, authorization, audit, and secure deployment.
9. **Testing and Quality Plan** — test layers, release gates, performance tests, and compatibility matrix.
10. **Implementation Roadmap** — small milestones, acceptance criteria, and dependency ordering.

---

## 25. Final Recommendation

Build this project as an **open-source Rust business application framework**, not as one universal business application and not as a replacement for the Rust web ecosystem.

Use established Rust libraries for infrastructure. Own the application runtime, module contracts, configuration system, reusable business engines, domain extensions, and developer experience.

Build a multi-crate Rust workspace and a modular, composable framework runtime. Support a unified runtime, separated workload processes, and selective independent services as deployment compositions. Keep the simplest composition easy to run, and do not impose distributed infrastructure unless a real requirement calls for it.

Validate the design with Restaurant POS and Pharmacy Management before expanding into additional business domains.

The strongest version of this project will not be the one with the largest initial feature list. It will be the one with clear module boundaries, reliable data and security guarantees, useful developer contracts, and a practical path for independent developers to build on top of it.

---

**Document note:** This is a product vision and architecture baseline. It is not a guarantee of legal name availability, trademark clearance, performance capacity, regulatory compliance, or compatibility of any particular dependency version. Those items require separate verification before release or production use.
