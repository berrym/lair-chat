# Transport Refactoring Flowcharts

This document contains comprehensive ASCII flowcharts documenting the transport module refactoring architecture, migration strategy, and system flows.

## Overview

These flowcharts illustrate the complete transport refactoring implementation, including:
- Migration decision logic and runtime architecture switching
- Connection establishment processes for both legacy and new architectures
- Error handling and propagation throughout the system
- Testing strategy and execution flow
- Production deployment strategy with gradual rollout
- Overall program architecture and data flow

## 1. Migration Decision Flowchart

This flowchart shows how the system decides between using the legacy global state architecture or the new ConnectionManager architecture based on environment variables and configuration.

```
                     ┌─────────────────────┐
                     │ Application Startup │
                     └──────────┬──────────┘
                                │
                                ▼
         ┌──────────────────────────────────────────────────────┐
         │ Check Environment Variable                           │
         │ LAIR_CHAT_USE_NEW_TRANSPORT                         │
         └─────────┬──────────┬─────────────┬─────────────────┘
                   │          │             │
                "true"     "false"      Not Set
                   │          │             │
                   ▼          ▼             ▼
         ┌─────────────┐  ┌─────────────┐  ┌─────────────────┐
         │   Enable    │  │   Enable    │  │ Check Migration │
         │     New     │  │   Legacy    │  │ Config auto_det │
         │ Architecture│  │Architecture │  └─────┬───────────┘
         └──────┬──────┘  └──────┬──────┘        │
                │                │           ┌───┴───┐
                │                │        true│     │false
                │                │            │     │
                │                │            ▼     ▼
                │                │    ┌───────────┐ ┌──────────────┐
                │                │    │Check Def. │ │Use Configured│
                │                │    │Config use │ │    Value     │
                │                │    │_new_arch  │ └──────┬───────┘
                │                │    └─────┬─────┘        │
                │                │          │              │
                │                │       ┌──┴──┐           │
                │                │    true│   │false       │
                │                │        │   │            │
                ▼                ▼        ▼   ▼            ▼
         ┌──────────────────────────────────────────────────────┐
         │              Initialize System                       │
         └─────────┬──────────────────────┬─────────────────────┘
                   │                      │
           New Architecture       Legacy Architecture
                   │                      │
                   ▼                      ▼
    ┌──────────────────────────┐ ┌─────────────────────┐
    │Initialize ConnectionMgr  │ │Use Global State Sys │
    └─────────┬────────────────┘ └─────────┬───────────┘
              │                            │
              ▼                            ▼
    ┌─────────────────┐           ┌─────────────────┐
    │Register         │           │Direct Global    │
    │TuiObserver      │           │State Access     │
    └─────┬───────────┘           └─────┬───────────┘
          │                             │
          ▼                             ▼
    ┌─────────────────┐           ┌─────────────────┐
    │Register         │           │Legacy UI        │
    │CompatObserver   │           │Feedback         │
    └─────┬───────────┘           └─────┬───────────┘
          │                             │
          ▼                             ▼
    ┌─────────────────┐           ┌─────────────────┐
    │Enhanced UI      │           │                 │
    │Feedback         │           │                 │
    └─────┬───────────┘           │                 │
          │                       │                 │
          ▼                       ▼                 │
    ┌─────────────────┐           │                 │
    │Sync with Global │           │                 │
    │State            │           │                 │
    └─────┬───────────┘           │                 │
          │                       │                 │
          └───────┬───────────────┴─────────────────┘
                  │
                  ▼
         ┌─────────────────────┐
         │ Ready for Connection│
         └─────────┬───────────┘
                   │
                   ▼
         ┌─────────────────────┐      ┌─────────────────┐
         │Runtime Switch       │ Yes  │Call migrate_    │
         │Needed?              ├─────▶│connection or    │
         └─────────┬───────────┘      │rollback_to_leg  │
                   │ No               └─────┬───────────┘
                   ▼                        │
         ┌─────────────────────┐            ▼
         │Continue Operation   │   ┌─────────────────┐
         └─────────────────────┘   │Graceful Arch    │
                                   │Switch           │
                                   └─────┬───────────┘
                                         │
                                         └──────┐
                   ┌─────────────────────┐      │
                   │Normal Operation     │◀─────┘
                   └─────────────────────┘
```

## 2. Connection Establishment Flow

This flowchart details the complete connection establishment process, showing how both the new ConnectionManager and legacy systems handle connections, including TCP setup, encryption handshakes, and observer notifications.

```
           ┌─────────────────────────┐
           │User Initiates Connection│
           └─────────┬───────────────┘
                     │
                     ▼
           ┌─────────────────────────┐
           │Using Migration Facade?  │
           └─────┬─────────────┬─────┘
                 │Yes          │No
                 ▼             ▼
    ┌─────────────────────┐   ┌─────────────┐
    │migration_facade::   │   │Direct API   │
    │connect_client       │   │Call         │
    └─────┬───────────────┘   └─────────────┘
          │
          ▼
    ┌─────────────────────┐
    │Check Architecture   │
    │Flag                 │
    └─────┬─────────┬─────┘
          │New      │Legacy
          ▼         ▼
 ┌─────────────┐   ┌─────────────────┐
 │connect_     │   │legacy_connect_  │
 │client_compat│   │client           │
 └─────┬───────┘   └─────┬───────────┘
       │                 │
       ▼                 │
 ┌─────────────────┐     │
 │Get/Create       │     │
 │ConnectionManager│     │
 └─────┬───────────┘     │
       │                 │
       ▼                 │
 ┌─────────────────┐     │
 │Setup TcpTransport│    │
 └─────┬───────────┘     │
       │                 │
       ▼                 │
 ┌─────────────────┐     │
 │Setup AesGcm     │     │
 │Encryption       │     │
 └─────┬───────────┘     │
       │                 │
       ▼                 │
 ┌─────────────────┐     │
 │Register         │     │
 │Observers        │     │
 └─────┬───────────┘     │
       │                 │
       ▼                 │
 ┌─────────────────┐     │
 │ConnectionManager│     │
 │.connect()       │     │
 └─────┬───────────┘     │
       │                 │
       ▼                 │
 ┌─────────────────┐     │
 │TcpTransport     │     │
 │.connect()       │     │
 └─────┬───────────┘     │
       │                 │
       ▼                 │
 ┌─────────────────┐     │
 │TCP Connection   │     │
 │Success?         │     │
 └─────┬─────┬─────┘     │
       │Yes  │No         │
       ▼     ▼           │
 ┌─────────┐ ┌─────────┐ │
 │Perform  │ │Connection│ │
 │X25519   │ │Error    │ │
 │Handshake│ └─────┬───┘ │
 └─────┬───┘       │     │
       │           │     │
       ▼           │     │
 ┌─────────────┐   │     │
 │Handshake    │   │     │
 │Success?     │   │     │
 └─────┬─┬─────┘   │     │
       │Y│N        │     │
       ▼ ▼         │     │
 ┌─────────┐ ┌─────┴───┐ │
 │Derive   │ │Handshake│ │
 │AES Key  │ │Error    │ │
 └─────┬───┘ └─────┬───┘ │
       │           │     │
       ▼           │     │
 ┌─────────────┐   │     │
 │Update Status│   │     │
 │to CONNECTED │   │     │
 └─────┬───────┘   │     │
       │           │     │
       ▼           │     │
 ┌─────────────┐   │     │
 │Notify       │   │     │
 │Observers    │   │     │
 └─────┬───────┘   │     │
       │           │     │
       ▼           │     │
 ┌─────────────┐   │     │
 │Start Message│   │     │
 │Loop         │   │     │
 └─────┬───────┘   │     │
       │           │     │
       │           │     │
       │           │     ▼
       │      ┌─────────────────┐
       │      │Legacy TCP       │
       │      │Connection       │
       │      └─────┬───────────┘
       │            │
       │            ▼
       │      ┌─────────────────┐
       │      │Legacy Key       │
       │      │Exchange         │
       │      └─────┬───────────┘
       │            │
       │            ▼
       │      ┌─────────────────┐
       │      │Legacy Message   │
       │      │Loop             │
       │      └─────┬───────────┘
       │            │
       │            │
       │       ┌────┴──────────┐
       │       │Error Handling │
       │       └────┬──────────┘
       │            │
       │            ▼
       │       ┌─────────────────┐
       │       │Notify Error     │
       │       │Observers        │
       │       └─────┬───────────┘
       │             │
       │             ▼
       │       ┌─────────────────┐
       │       │Connection Failed│
       │       └─────────────────┘
       │
       ▼
 ┌─────────────────┐
 │Ready for Messages│
 └─────────────────┘
```

## 3. Error Handling Flowchart

This flowchart illustrates how errors are caught, categorized, propagated through the system, and displayed to users. It shows the differences between legacy and new architecture error handling.

```
                    ┌─────────────┐
                    │Error Occurs │
                    └──────┬──────┘
                           │
                           ▼
                    ┌─────────────┐
                    │Error Source?│
                    └─┬─┬─┬─┬─────┘
                      │ │ │ │
           Transport──┘ │ │ │
           Encryption───┘ │ │
           Connection─────┘ │
           Protocol────────┘
                      │ │ │ │
                      ▼ ▼ ▼ ▼
        ┌─────────────┐ ┌─────────────┐ ┌─────────────┐ ┌─────────────┐
        │TransportErr │ │EncryptionErr│ │ConnectionErr│ │ProtocolErr  │
        └──────┬──────┘ └──────┬──────┘ └──────┬──────┘ └──────┬──────┘
               │               │               │               │
               ▼               ▼               ▼               ▼
        ┌─────────────┐ ┌─────────────┐ ┌─────────────┐ ┌─────────────┐
        │Error Type?  │ │Encryption   │ │Network      │ │Protocol     │
        └─┬─┬─┬───────┘ │Failure      │ │Issue        │ │Violation    │
          │ │ │         └──────┬──────┘ └──────┬──────┘ └──────┬──────┘
       IO─┘ │ │                │               │               │
       Timeout─┘ │              │               │               │
       Auth──────┘              │               │               │
          │ │ │                 │               │               │
          ▼ ▼ ▼                 ▼               ▼               ▼
   ┌──────────┐ ┌──────────┐ ┌──────────────────────────────────────────┐
   │Wrap      │ │Timeout   │ │           Create TransportError            │
   │std::io   │ │Error     │ └──────────────┬───────────────────────────┘
   │Error     │ └─────┬────┘                │
   └─────┬────┘       │                     │
         │            │                     │
         └─────┬──────┴─────────────────────┘
               │
               ▼
        ┌─────────────────┐
        │Using New        │
        │Architecture?    │
        └─────┬─────┬─────┘
              │Yes  │No
              ▼     ▼
    ┌─────────────┐ ┌─────────────────┐
    │Notify       │ │Direct Global    │
    │Connection   │ │State Update     │
    │Observers    │ └─────┬───────────┘
    └──────┬──────┘       │
           │              │
           ▼              │
    ┌─────────────┐       │
    │TuiObserver  │       │
    │.on_error()  │       │
    └──────┬──────┘       │
           │              │
           ▼              │
    ┌─────────────┐       │
    │Add "ERROR:  │       │
    │message" to  │       │
    │UI           │       │
    └──────┬──────┘       │
           │              │
           ▼              │
    ┌─────────────┐       │
    │Compatibility│       │
    │Observer     │       │
    │.on_error()  │       │
    └──────┬──────┘       │
           │              │
           ▼              │
    ┌─────────────┐       │
    │Update Global│       │
    │State        │       │
    └──────┬──────┘       │
           │              │
           ▼              │
    ┌─────────────┐       │
    │Add "ERROR:  │       │
    │message" to  │       │
    │UI           │       │
    └──────┬──────┘       │
           │              │
           │              ▼
           │       ┌─────────────┐
           │       │Add "Error:  │
           │       │message" to  │
           │       │UI           │
           │       └─────┬───────┘
           │             │
           └─────┬───────┴────────────┐
                 │                    │
                 ▼                    ▼
         ┌─────────────────┐  ┌─────────────────┐
         │User Sees Error  │  │Legacy Systems   │
         └─────────┬───────┘  │Updated          │
                   │          └─────────────────┘
                   │
                   ▼
         ┌─────────────────┐
         │Recoverable      │     ┌─────────────┐
         │Error?           ├─Yes─▶│Retry Logic  │
         └─────────┬───────┘     └─────┬───────┘
                   │No                 │
                   ▼                   ▼
         ┌─────────────────┐     ┌─────────────┐
         │Disconnect       │     │Exponential  │
         └─────────┬───────┘     │Backoff      │
                   │             └─────┬───────┘
                   ▼                   │
         ┌─────────────────┐           ▼
         │Clean Shutdown   │     ┌─────────────┐
         └─────────┬───────┘     │Retry        │
                   │             │Connection   │
                   ▼             └─────────────┘
         ┌─────────────────┐
         │Reset State      │
         └─────────────────┘
```

## 4. Test Execution Flow

This flowchart shows the comprehensive testing strategy, including unit tests, integration tests, and migration compatibility tests. It demonstrates how the 30+ tests are organized and executed.

```
                    ┌─────────────┐
                    │ Run Tests   │
                    └──────┬──────┘
                           │
                           ▼
                    ┌─────────────┐
                    │Test Category│
                    └─┬─────┬───┬─┘
                      │     │   │
                Unit──┘     │   │
                Integration─┘   │
                Compatibility──┘
                      │     │   │
                      ▼     ▼   ▼
        ┌─────────────┐ ┌─────┐ ┌─────────────┐
        │Component    │ │E2E  │ │Migration    │
        │Tests        │ │Tests│ │Tests        │
        └──────┬──────┘ └──┬──┘ └──────┬──────┘
               │           │           │
     ┌─────────┼───────────┼───────────┼─────────┐
     │         │           │           │         │
     ▼         ▼           ▼           ▼         ▼
┌─────────┐ ┌─────────┐ ┌─────────┐ ┌─────────┐ ┌─────────┐
│ConnMgr  │ │TcpTrans │ │Real     │ │Handshake│ │Arch     │
│Tests    │ │Tests    │ │Conn     │ │Integr   │ │Switch   │
└────┬────┘ └────┬────┘ │Tests    │ └────┬────┘ └────┬────┘
     │           │      └────┬────┘      │           │
     ▼           ▼           │           ▼           ▼
┌─────────┐ ┌─────────┐     │      ┌─────────┐ ┌─────────┐
│AesGcm   │ │TuiObs   │     │      │Message  │ │State    │
│Tests    │ │Tests    │     │      │Flow     │ │Sync     │
└────┬────┘ └────┬────┘     │      │Tests    │ └────┬────┘
     │           │          │      └────┬────┘      │
     ▼           ▼          ▼           │           ▼
┌─────────┐ ┌─────────┐ ┌─────────┐     │      ┌─────────┐
│Mock     │ │UI Inter │ │System   │     │      │Env Var  │
│Deps     │ │Tests    │ │Integr   │     │      │Tests    │
└────┬────┘ └────┬────┘ └────┬────┘     │      └────┬────┘
     │           │           │          │           │
     ▼           ▼           ▼          ▼           ▼
┌─────────┐ ┌─────────┐ ┌─────────┐ ┌─────────┐ ┌─────────┐
│Mock     │ │Crypto   │ │         │ │         │ │Migration│
│Network  │ │Test     │ │         │ │         │ │Validat  │
└────┬────┘ │Vectors  │ │         │ │         │ └────┬────┘
     │      └────┬────┘ │         │ │         │      │
     │           │      │         │ │         │      │
     └─────┬─────┴──────┴─────────┴─┴─────────┴──────┘
           │
           ▼
    ┌─────────────┐
    │Isolated     │
    │Testing      │
    └──────┬──────┘
           │
           ▼
    ┌─────────────┐      ┌─────────────┐
    │All Unit     │ Yes  │Unit Tests ✓ │
    │Tests Pass?  ├─────▶└─────────────┘
    └──────┬──────┘
           │No
           ▼
    ┌─────────────┐
    │Fix Unit     │─────┐
    │Issues       │     │
    └─────────────┘     │
           │            │
           └────────────┘
           
    Similar flow for Integration and Migration tests...
    
           ┌─────────────┐
           │Test Report  │
           └──────┬──────┘
                  │
                  ▼
           ┌─────────────┐
           │30+ Tests    │
           │Passing      │
           └─────────────┘
```

## 5. Deployment Strategy Flowchart

This flowchart outlines the five-phase production deployment strategy, from conservative rollout to complete migration, including rollback procedures and monitoring.

```
    ┌─────────────────┐
    │Deployment       │
    │Planning         │
    └─────────┬───────┘
              │
              ▼
    ┌─────────────────┐
    │Phase A:         │
    │Conservative     │
    │Rollout          │
    └─────────┬───────┘
              │
              ▼
    ┌─────────────────────────┐
    │Deploy with Legacy Mode  │
    └─────────┬───────────────┘
              │
              ▼
    ┌─────────────────────────┐
    │Set LAIR_CHAT_USE_NEW_   │
    │TRANSPORT=false          │
    └─────────┬───────────────┘
              │
              ▼
    ┌─────────────────────────┐
    │Verify Existing          │
    │Functionality            │
    └─────────┬───────────────┘
              │
              ▼
    ┌─────────────────────────┐      ┌─────────────────┐
    │Production Stable?       │ No   │Fix Issues       │
    └─────────┬───────────────┘ ────▶└─────┬───────────┘
              │Yes                         │
              ▼                            │
    ┌─────────────────────────┐            │
    │Phase B: Staging         │            │
    │Validation               │            │
    └─────────┬───────────────┘            │
              │                            │
              ▼                            │
    ┌─────────────────────────┐            │
    │Enable New Architecture  │            │
    │in Staging               │            │
    └─────────┬───────────────┘            │
              │                            │
              ▼                            │
    ┌─────────────────────────┐            │
    │Set LAIR_CHAT_USE_NEW_   │            │
    │TRANSPORT=true           │            │
    └─────────┬───────────────┘            │
              │                            │
              ▼                            │
    ┌─────────────────────────┐            │
    │Run Full Test Suite      │            │
    └─────────┬───────────────┘            │
              │                            │
              ▼                            │
    ┌─────────────────────────┐            │
    │Staging Tests Pass?      │ No         │
    └─────────┬───────────────┘ ──────────┘
              │Yes
              ▼
    ┌─────────────────────────┐
    │Performance Baseline     │
    └─────────┬───────────────┘
              │
              ▼
    ┌─────────────────────────┐      ┌─────────────────┐
    │Performance Acceptable?  │ No   │Optimize or      │
    └─────────┬───────────────┘ ────▶│Rollback         │
              │Yes                   └─────────────────┘
              ▼
    ┌─────────────────────────┐
    │Phase C: Gradual         │
    │Production Rollout       │
    └─────────┬───────────────┘
              │
              ▼
    ┌─────────────────────────┐
    │Enable for Small         │
    │User Subset              │
    └─────────┬───────────────┘
              │
              ▼
    ┌─────────────────────────┐
    │Monitor Key Metrics      │
    └─────┬─────┬─────┬───────┘
          │     │     │
          ▼     ▼     ▼
    ┌─────────┐ ┌─────┐ ┌─────────┐
    │Conn     │ │Error│ │Perform  │
    │Success  │ │Rates│ │Metrics  │
    │Rate     │ └─────┘ └─────────┘
    └─────────┘   │       │
          │       │       │
          └───────┼───────┘
                  │
                  ▼
    ┌─────────────────────────┐
    │Metrics Healthy?         │
    └─────────┬───────────────┘
              │
         ┌────┴────┐
         │No       │Yes
         ▼         ▼
    ┌─────────┐   ┌─────────────────┐
    │Rollback │   │Expand to More   │
    │Subset   │   │Users            │
    └────┬────┘   └─────┬───────────┘
         │              │
         ▼              ▼
    ┌─────────┐   ┌─────────────────┐
    │Investig │   │Full Rollout     │ No
    │ate      │   │Ready?           ├─────┐
    │Issues   │   └─────┬───────────┘     │
    └─────────┘         │Yes              │
         │              ▼                 │
         └─────────┐    ┌─────────────────┐ │
                   │    │Phase D:         │ │
                   │    │Complete         │ │
                   │    │Migration        │ │
                   │    └─────┬───────────┘ │
                   │          │             │
                   │          ▼             │
                   │    ┌─────────────────┐ │
                   │    │Enable New Arch  │ │
                   │    │Globally         │ │
                   │    └─────┬───────────┘ │
                   │          │             │
                   │          ▼             │
                   │    ┌─────────────────┐ │
                   │    │Monitor All      │ │
                   │    │Systems          │ │
                   │    └─────┬───────────┘ │
                   │          │             │
                   │          ▼             │
                   │    ┌─────────────────┐ │
                   │    │Migration        │ │
                   │    │Successful?      │ │
                   │    └─────┬─────┬─────┘ │
                   │          │No   │Yes    │
                   │          ▼     ▼       │
                   │    ┌─────────┐ ┌───────┴────┐
                   │    │Emergency│ │Phase E:    │
                   │    │Rollback │ │Cleanup     │
                   │    └────┬────┘ └┬───────────┘
                   │         │       │
                   │         ▼       ▼
                   │    ┌─────────┐ ┌─────────────┐
                   │    │Set      │ │Remove       │
                   │    │LAIR_... │ │Compatibility│
                   │    │=false   │ │Layer        │
                   │    └────┬────┘ └─────┬───────┘
                   │         │            │
                   │         ▼            ▼
                   │    ┌─────────┐ ┌─────────────┐
                   │    │Post-    │ │Update       │
                   │    │Mortem   │ │Documentation│
                   │    │Analysis │ └─────┬───────┘
                   │    └─────────┘       │
                   │                      ▼
                   │               ┌─────────────┐
                   │               │Archive      │
                   │               │Legacy Code  │
                   │               └─────┬───────┘
                   │                     │
                   │                     ▼
                   │               ┌─────────────┐
                   │               │Migration    │
                   │               │Complete     │
                   │               └─────────────┘
                   │
                   └──────────────────────────────────┘
```

## 6. Overall Program Architecture Flow

This flowchart provides a high-level view of how the entire Lair Chat application works with the migration system, showing the complete data flow from startup to user interactions.

```
                ┌─────────────────────────┐
                │Lair Chat Application    │
                │Start                    │
                └─────────┬───────────────┘
                          │
                          ▼
                ┌─────────────────────────┐
                │Initialize Migration     │
                │System                   │
                └─────────┬───────────────┘
                          │
                          ▼
                ┌─────────────────────────┐
                │Migration Configuration  │
                └─┬─────────┬─────────┬───┘
                  │         │         │
             Env Var     Manual    Default
                  │      Config      │
                  ▼         │        ▼
        ┌─────────────┐     │  ┌─────────────┐
        │Check LAIR_  │     │  │Conservative │
        │CHAT_USE_NEW_│     │  │Legacy Mode  │
        │TRANSPORT    │     │  └─────┬───────┘
        └─────┬───────┘     │        │
              │             │        │
              ▼             ▼        │
        ┌─────────────┐ ┌─────────┐  │
        │Value?       │ │Use      │  │
        └─┬─────────┬─┘ │Migration│  │
          │true     │   │Config   │  │
          ▼         │   └────┬────┘  │
    ┌─────────┐     │        │       │
    │New Arch │     │        ▼       │
    │Path     │     │   ┌─────────┐  │
    └────┬────┘     │   │         │  │
         │          │   │         │  │
         │          ▼   ▼         ▼  ▼
         │    ┌─────────────┐ ┌─────────────┐
         │    │Legacy Arch  │ │             │
         │    │Path         │ │             │
         │    └─────┬───────┘ │             │
         │          │         │             │
         │          │         │             │
         └──────────┼─────────┼─────────────┘
                    │         │
                    ▼         ▼
        ┌─────────────────────┬─────────────────────┐
        │                     │                     │
        ▼                     ▼                     ▼
┌──────────────┐      ┌──────────────┐      ┌──────────────┐
│Initialize    │      │Use Global    │      │              │
│Connection    │      │State System  │      │              │
│Manager       │      └──────┬───────┘      │              │
└──────┬───────┘             │              │              │
       │                     ▼              │              │
       ▼              ┌──────────────┐       │              │
┌──────────────┐      │Legacy TCP    │       │              │
│Create        │      │Implementation│       │              │
│Transport     │      └──────┬───────┘       │              │
│Layer         │             │               │              │
└──────┬───────┘             ▼               │              │
       │              ┌──────────────┐       │              │
       ▼              │Legacy        │       │              │
┌──────────────┐      │Encryption    │       │              │
│TcpTransport  │      └──────┬───────┘       │              │
│with async/   │             │               │              │
│await         │             ▼               │              │
└──────┬───────┘      ┌──────────────┐       │              │
       │              │Direct Global │       │              │
       ▼              │State Access  │       │              │
┌──────────────┐      └──────┬───────┘       │              │
│Create        │             │               │              │
│Encryption    │             ▼               │              │
│Layer         │      ┌──────────────┐       │              │
└──────┬───────┘      │Legacy UI     │       │              │
       │              │Feedback      │       │              │
       ▼              └──────┬───────┘       │              │
┌──────────────┐             │               │              │
│AesGcm        │             │               │              │
│Encryption    │             │               │              │
│with X25519   │             │               │              │
└──────┬───────┘             │               │              │
       │                     │               │              │
       ▼                     │               │              │
┌──────────────┐             │               │              │
│Register      │             │               │              │
│Observers     │             │               │              │
└──┬─────────┬─┘             │               │              │
   │         │               │               │              │
   ▼         ▼               │               │              │
┌─────────┐ ┌─────────────┐  │               │              │
│TuiObs - │ │Compatibility│  │               │              │
│Enhanced │ │Observer -   │  │               │              │
│UI       │ │State Sync   │  │               │              │
└────┬────┘ └─────┬───────┘  │               │              │
     │            │          │               │              │
     ▼            ▼          │               │              │
┌─────────────┐ ┌─────────────┐              │              │
│Professional│ │Global State │              │              │
│UI Feedback │ │Synchronized │              │              │
└─────┬───────┘ └─────┬───────┘              │              │
      │               │                      │              │
      │               │                      │              │
      └───────┬───────┴──────────────────────┴──────────────┘
              │
              ▼
    ┌─────────────────────┐
    │User Interface       │
    └─────────┬───────────┘
              │
              ▼
    ┌─────────────────────┐
    │TUI with Home        │
    │Component            │
    └─────────┬───────────┘
              │
              ▼
    ┌─────────────────────┐
    │User Input Handling  │
    └─────────┬───────────┘
              │
              ▼
    ┌─────────────────────┐
    │User Action?         │
    └─┬─────┬─────┬─────┬─┘
      │     │     │     │
   Connect Send Disconn Scroll
      │   Message  │     │
      ▼     │      ▼     ▼
┌─────────┐ │ ┌─────────┐ ┌─────────┐
│Connect  │ │ │Disconnect│ │UI       │
│Request  │ │ │         │ │Navigation│
└────┬────┘ │ └────┬────┘ └─────────┘
     │      │      │
     ▼      ▼      ▼
┌─────────────────────┐ ┌─────────────────────┐
│Architecture Active? │ │Architecture Active? │
└─┬─────────────────┬─┘ └─┬─────────────────┬─┘
  │New            │Leg   │New            │Leg
  ▼               ▼      ▼               ▼
┌─────────┐ ┌─────────┐ ┌─────────┐ ┌─────────┐
│ConnMgr  │ │legacy_  │ │ConnMgr  │ │Legacy   │
│.connect │ │connect_ │ │.send_   │ │Message  │
└────┬────┘ │client   │ │message  │ │Send     │
     │      └────┬────┘ └────┬────┘ └────┬────┘
     ▼           │           │           │
┌─────────┐      │           ▼           │
│TCP +    │      │      ┌─────────┐      │
│X25519 + │      │      │Encrypted│      │
│AES-GCM  │      │      │Message  │      │
└────┬────┘      │      │Transport│      │
     │           │      └────┬────┘      │
     │           ▼           │           ▼
     │      ┌─────────┐      │      ┌─────────┐
     │      │Legacy   │      │      │         │
     │      │TCP +    │      │      │         │
     │      │Encrypt  │      │      │         │
     │      └────┬────┘      │      │         │
     │           │           │      │         │
     └───────────┼───────────┼──────┼─────────┘
                 │           │      │
                 ▼           ▼      ▼
           ┌─────────────────────────────┐
           │Message Loop Active          │
           └─────────┬───────────────────┘
                     │
                     ▼
           ┌─────────────────────────────┐
           │Receive Messages             │
           └─────────┬───────────────────┘
                     │
                     ▼
           ┌─────────────────────────────┐
           │Decrypt and Display          │
           └─────────┬───────────────────┘
                     │
                     │ ┌─────────────────┐
                     └▶│Connection Closed│
                       └─────┬───────────┘
                             │
                             ▼
                       ┌─────────────────┐
                       │Application Exit?│
                       └─┬─────────────┬─┘
                         │No           │Yes
                         │             ▼
                         │       ┌─────────┐
                         │       │Cleanup  │
                         │       │and Exit │
                         │       └─────────┘
                         │
                         └─────────────────┐
                                          │
                         ┌────────────────┘
                         │
                         ▼
                   ┌─────────────┐
                   │Continue     │
                   │Operation    │
                   └─────────────┘
```

## Usage Instructions

These ASCII flowcharts are designed for maximum portability and can be used in:

1. **Any Text Editor**: Notepad, Vi, Emacs, VS Code, etc.
2. **Terminal/Console**: Direct viewing in command line interfaces
3. **Email**: Plain text emails and communication
4. **Code Comments**: Embedded directly in source code
5. **Documentation**: Any documentation system that supports monospace text
6. **Chat/Messaging**: Slack, Discord, Teams, etc.
7. **Presentations**: Code slides or technical presentations
8. **Version Control**: Git commit messages, issue descriptions
9. **README Files**: Simple markdown or plain text READMEs
10. **Legacy Systems**: Any system that supports basic text display

**Viewing Tips:**
- Use a monospace font (Courier, Consolas, Monaco, etc.)
- Ensure adequate terminal width (80+ characters recommended)
- For best results, use 120+ character width displays

## Architecture Benefits Visualized

These flowcharts demonstrate key benefits of the refactored architecture:

- **Flexibility**: Runtime switching between implementations
- **Reliability**: Comprehensive error handling and recovery  
- **Testability**: Clear separation enabling thorough testing
- **Maintainability**: Well-defined interfaces and responsibilities
- **Deployability**: Safe, gradual rollout with instant rollback
- **Portability**: Documentation works everywhere, no special tools needed

## Related Documentation

- `TRANSPORT_REFACTORING_PLAN.md`: Detailed implementation plan
- `src/client/compatibility_layer.rs`: Bridge implementation  
- `src/client/migration_facade.rs`: Feature flag system
- `src/client/connection_manager.rs`: New architecture core
