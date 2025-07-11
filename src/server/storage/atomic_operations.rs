//! Atomic Transaction Operations Implementation
//!
//! This module provides concrete implementations of atomic database operations
//! for the lair-chat server, including invitation management, room operations,
//! and user management with full rollback support.

use super::{current_timestamp, generate_id, models::*, transactions::*, StorageError};
use async_trait::async_trait;
use sqlx::Row;

/// Atomic operations implementation
pub struct AtomicOperations {
    // This will be used for non-transactional operations if needed
    _phantom: std::marker::PhantomData<()>,
}

impl AtomicOperations {
    pub fn new() -> Self {
        Self {
            _phantom: std::marker::PhantomData,
        }
    }
}

impl Default for AtomicOperations {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl TransactionOperations for AtomicOperations {
    /// Create invitation with membership atomically
    async fn create_invitation_with_membership(
        &self,
        transaction: &mut Transaction<'_>,
        invitation: Invitation,
        membership: RoomMembership,
    ) -> TransactionResult<(Invitation, RoomMembership)> {
        transaction.record_operation("create_invitation_with_membership");

        // Validate that the room exists
        let room_check = sqlx::query("SELECT id FROM rooms WHERE id = ?")
            .bind(&invitation.room_id)
            .fetch_optional(&mut **transaction.inner_mut())
            .await
            .map_err(|e| {
                TransactionError::StorageError(StorageError::QueryError {
                    message: format!("Failed to check room existence: {}", e),
                })
            })?;

        if room_check.is_none() {
            return Err(TransactionError::StorageError(StorageError::NotFound {
                entity: "Room".to_string(),
                id: invitation.room_id.clone(),
            }));
        }

        // Validate that the inviter has permission to invite
        let inviter_membership =
            sqlx::query("SELECT role FROM room_memberships WHERE room_id = ? AND user_id = ?")
                .bind(&invitation.room_id)
                .bind(&invitation.sender_user_id)
                .fetch_optional(&mut **transaction.inner_mut())
                .await
                .map_err(|e| {
                    TransactionError::StorageError(StorageError::QueryError {
                        message: format!("Failed to check inviter permissions: {}", e),
                    })
                })?;

        if let Some(row) = inviter_membership {
            let role: String = row.get("role");
            let inviter_role = match role.as_str() {
                "owner" => RoomRole::Owner,
                "admin" => RoomRole::Admin,
                "member" => RoomRole::Member,
                _ => {
                    return Err(TransactionError::StorageError(
                        StorageError::ValidationError {
                            field: "role".to_string(),
                            message: "Invalid role".to_string(),
                        },
                    ))
                }
            };

            // Only owners and admins can invite
            if !matches!(inviter_role, RoomRole::Owner | RoomRole::Admin) {
                return Err(TransactionError::StorageError(
                    StorageError::ValidationError {
                        field: "inviter_role".to_string(),
                        message: "Insufficient permissions to invite".to_string(),
                    },
                ));
            }
        } else {
            return Err(TransactionError::StorageError(
                StorageError::ValidationError {
                    field: "inviter_id".to_string(),
                    message: "Inviter is not a member of the room".to_string(),
                },
            ));
        }

        // Check if user is already a member
        let existing_membership =
            sqlx::query("SELECT id FROM room_memberships WHERE room_id = ? AND user_id = ?")
                .bind(&invitation.room_id)
                .bind(&invitation.recipient_user_id)
                .fetch_optional(&mut **transaction.inner_mut())
                .await
                .map_err(|e| {
                    TransactionError::StorageError(StorageError::QueryError {
                        message: format!("Failed to check existing membership: {}", e),
                    })
                })?;

        if existing_membership.is_some() {
            return Err(TransactionError::StorageError(
                StorageError::DuplicateError {
                    entity: "RoomMembership".to_string(),
                    message: "User is already a member of the room".to_string(),
                },
            ));
        }

        // Check if invitation already exists
        let existing_invitation = sqlx::query("SELECT id FROM invitations WHERE room_id = ? AND recipient_user_id = ? AND status = 'pending'")
            .bind(&invitation.room_id)
            .bind(&invitation.recipient_user_id)
            .fetch_optional(&mut **transaction.inner_mut())
            .await
            .map_err(|e| TransactionError::StorageError(StorageError::QueryError {
                message: format!("Failed to check existing invitation: {}", e),
            }))?;

        if existing_invitation.is_some() {
            return Err(TransactionError::StorageError(
                StorageError::DuplicateError {
                    entity: "Invitation".to_string(),
                    message: "Pending invitation already exists".to_string(),
                },
            ));
        }

        // Insert invitation
        sqlx::query(
            "INSERT INTO invitations (id, room_id, sender_user_id, recipient_user_id, status, created_at, expires_at) VALUES (?, ?, ?, ?, ?, ?, ?)"
        )
        .bind(&invitation.id)
        .bind(&invitation.room_id)
        .bind(&invitation.sender_user_id)
        .bind(&invitation.recipient_user_id)
        .bind(match invitation.status {
            InvitationStatus::Pending => "pending",
            InvitationStatus::Accepted => "accepted",
            InvitationStatus::Declined => "declined",
            InvitationStatus::Expired => "expired",
            InvitationStatus::Revoked => "revoked",
        })
        .bind(invitation.created_at as i64)
        .bind(invitation.expires_at.unwrap_or(invitation.created_at + 3600) as i64)
        .execute(&mut **transaction.inner_mut())
        .await
        .map_err(|e| TransactionError::StorageError(StorageError::QueryError {
            message: format!("Failed to insert invitation: {}", e),
        }))?;

        // Create pending membership (will be activated when invitation is accepted)
        sqlx::query(
            "INSERT INTO room_memberships (id, room_id, user_id, role, joined_at, status) VALUES (?, ?, ?, ?, ?, ?)"
        )
        .bind(&membership.id)
        .bind(&membership.room_id)
        .bind(&membership.user_id)
        .bind(match membership.role {
            RoomRole::Owner => "owner",
            RoomRole::Admin => "admin",
            RoomRole::Member => "member",
        })
        .bind(membership.joined_at as i64)
        .bind("pending")
        .execute(&mut **transaction.inner_mut())
        .await
        .map_err(|e| TransactionError::StorageError(StorageError::QueryError {
            message: format!("Failed to insert membership: {}", e),
        }))?;

        Ok((invitation, membership))
    }

    /// Update invitation and membership atomically
    async fn update_invitation_and_membership(
        &self,
        transaction: &mut Transaction<'_>,
        invitation_id: &str,
        new_status: InvitationStatus,
        membership: RoomMembership,
    ) -> TransactionResult<(Invitation, RoomMembership)> {
        transaction.record_operation("update_invitation_and_membership");

        // Get current invitation
        let invitation_row = sqlx::query("SELECT * FROM invitations WHERE id = ?")
            .bind(invitation_id)
            .fetch_optional(&mut **transaction.inner_mut())
            .await
            .map_err(|e| {
                TransactionError::StorageError(StorageError::QueryError {
                    message: format!("Failed to fetch invitation: {}", e),
                })
            })?;

        let invitation_row = invitation_row.ok_or_else(|| {
            TransactionError::StorageError(StorageError::NotFound {
                entity: "Invitation".to_string(),
                id: invitation_id.to_string(),
            })
        })?;

        // Parse current invitation
        let current_invitation = Invitation {
            id: invitation_row.get("id"),
            room_id: invitation_row.get("room_id"),
            sender_user_id: invitation_row.get("sender_user_id"),
            recipient_user_id: invitation_row.get("recipient_user_id"),
            invitation_type: InvitationType::RoomInvitation,
            status: match invitation_row.get::<String, _>("status").as_str() {
                "pending" => InvitationStatus::Pending,
                "accepted" => InvitationStatus::Accepted,
                "declined" => InvitationStatus::Declined,
                "expired" => InvitationStatus::Expired,
                "revoked" => InvitationStatus::Revoked,
                _ => {
                    return Err(TransactionError::StorageError(
                        StorageError::ValidationError {
                            field: "status".to_string(),
                            message: "Invalid invitation status".to_string(),
                        },
                    ))
                }
            },
            message: None,
            created_at: invitation_row.get::<i64, _>("created_at") as u64,
            expires_at: Some(invitation_row.get::<i64, _>("expires_at") as u64),
            responded_at: None,
            metadata: InvitationMetadata {
                sender_permissions: Vec::new(),
                recipient_permissions: Vec::new(),
                invitation_context: None,
                custom_fields: std::collections::HashMap::new(),
            },
        };

        // Validate state transition
        match (current_invitation.status, new_status) {
            (InvitationStatus::Pending, InvitationStatus::Accepted) => {
                // Valid transition
            }
            (InvitationStatus::Pending, InvitationStatus::Declined) => {
                // Valid transition
            }
            (InvitationStatus::Pending, InvitationStatus::Expired) => {
                // Valid transition
            }
            _ => {
                return Err(TransactionError::StorageError(
                    StorageError::ValidationError {
                        field: "status".to_string(),
                        message: format!(
                            "Invalid status transition from {:?} to {:?}",
                            current_invitation.status, new_status
                        ),
                    },
                ));
            }
        }

        // Update invitation status
        sqlx::query("UPDATE invitations SET status = ? WHERE id = ?")
            .bind(match new_status {
                InvitationStatus::Pending => "pending",
                InvitationStatus::Accepted => "accepted",
                InvitationStatus::Declined => "declined",
                InvitationStatus::Expired => "expired",
                InvitationStatus::Revoked => "revoked",
            })
            .bind(invitation_id)
            .execute(&mut **transaction.inner_mut())
            .await
            .map_err(|e| {
                TransactionError::StorageError(StorageError::QueryError {
                    message: format!("Failed to update invitation: {}", e),
                })
            })?;

        // Update membership status based on invitation status
        let membership_status = match new_status {
            InvitationStatus::Accepted => "active",
            InvitationStatus::Declined => "declined",
            InvitationStatus::Expired => "expired",
            InvitationStatus::Pending => "pending",
            InvitationStatus::Revoked => "revoked",
        };

        sqlx::query("UPDATE room_memberships SET status = ? WHERE room_id = ? AND user_id = ?")
            .bind(membership_status)
            .bind(&membership.room_id)
            .bind(&membership.user_id)
            .execute(&mut **transaction.inner_mut())
            .await
            .map_err(|e| {
                TransactionError::StorageError(StorageError::QueryError {
                    message: format!("Failed to update membership: {}", e),
                })
            })?;

        // If declined or expired, remove the membership
        if matches!(
            new_status,
            InvitationStatus::Declined | InvitationStatus::Expired
        ) {
            sqlx::query("DELETE FROM room_memberships WHERE room_id = ? AND user_id = ?")
                .bind(&membership.room_id)
                .bind(&membership.user_id)
                .execute(&mut **transaction.inner_mut())
                .await
                .map_err(|e| {
                    TransactionError::StorageError(StorageError::QueryError {
                        message: format!("Failed to delete membership: {}", e),
                    })
                })?;
        }

        let updated_invitation = Invitation {
            status: new_status,
            ..current_invitation
        };

        Ok((updated_invitation, membership))
    }

    /// Perform batch room operations atomically
    async fn batch_room_operations(
        &self,
        transaction: &mut Transaction<'_>,
        operations: Vec<RoomOperation>,
    ) -> TransactionResult<Vec<RoomOperationResult>> {
        transaction.record_operation("batch_room_operations");

        let mut results = Vec::new();

        for operation in operations {
            match operation {
                RoomOperation::CreateRoom(room) => {
                    sqlx::query(
                        "INSERT INTO rooms (id, name, description, is_public, created_at, created_by) VALUES (?, ?, ?, ?, ?, ?)"
                    )
                    .bind(&room.id)
                    .bind(&room.name)
                    .bind(&room.description)
                    .bind(room.is_public)
                    .bind(room.created_at as i64)
                    .bind(&room.created_by)
                    .execute(&mut **transaction.inner_mut())
                    .await
                    .map_err(|e| TransactionError::StorageError(StorageError::QueryError {
                        message: format!("Failed to create room: {}", e),
                    }))?;

                    results.push(RoomOperationResult::RoomCreated(room));
                }

                RoomOperation::UpdateRoom(room) => {
                    sqlx::query(
                        "UPDATE rooms SET name = ?, description = ?, is_public = ? WHERE id = ?",
                    )
                    .bind(&room.name)
                    .bind(&room.description)
                    .bind(room.is_public)
                    .bind(&room.id)
                    .execute(&mut **transaction.inner_mut())
                    .await
                    .map_err(|e| {
                        TransactionError::StorageError(StorageError::QueryError {
                            message: format!("Failed to update room: {}", e),
                        })
                    })?;

                    results.push(RoomOperationResult::RoomUpdated(room));
                }

                RoomOperation::DeleteRoom(room_id) => {
                    // Delete all memberships first
                    sqlx::query("DELETE FROM room_memberships WHERE room_id = ?")
                        .bind(&room_id)
                        .execute(&mut **transaction.inner_mut())
                        .await
                        .map_err(|e| {
                            TransactionError::StorageError(StorageError::QueryError {
                                message: format!("Failed to delete room memberships: {}", e),
                            })
                        })?;

                    // Delete all invitations
                    sqlx::query("DELETE FROM invitations WHERE room_id = ?")
                        .bind(&room_id)
                        .execute(&mut **transaction.inner_mut())
                        .await
                        .map_err(|e| {
                            TransactionError::StorageError(StorageError::QueryError {
                                message: format!("Failed to delete room invitations: {}", e),
                            })
                        })?;

                    // Delete all messages
                    sqlx::query("DELETE FROM messages WHERE room_id = ?")
                        .bind(&room_id)
                        .execute(&mut **transaction.inner_mut())
                        .await
                        .map_err(|e| {
                            TransactionError::StorageError(StorageError::QueryError {
                                message: format!("Failed to delete room messages: {}", e),
                            })
                        })?;

                    // Delete the room
                    sqlx::query("DELETE FROM rooms WHERE id = ?")
                        .bind(&room_id)
                        .execute(&mut **transaction.inner_mut())
                        .await
                        .map_err(|e| {
                            TransactionError::StorageError(StorageError::QueryError {
                                message: format!("Failed to delete room: {}", e),
                            })
                        })?;

                    results.push(RoomOperationResult::RoomDeleted(room_id));
                }

                RoomOperation::AddMember(membership) => {
                    sqlx::query(
                        "INSERT INTO room_memberships (id, room_id, user_id, role, joined_at, status) VALUES (?, ?, ?, ?, ?, ?)"
                    )
                    .bind(&membership.id)
                    .bind(&membership.room_id)
                    .bind(&membership.user_id)
                    .bind(match membership.role {
                        RoomRole::Owner => "owner",
                        RoomRole::Admin => "admin",
                        RoomRole::Member => "member",
                    })
                    .bind(membership.joined_at as i64)
                    .bind("active")
                    .execute(&mut **transaction.inner_mut())
                    .await
                    .map_err(|e| TransactionError::StorageError(StorageError::QueryError {
                        message: format!("Failed to add member: {}", e),
                    }))?;

                    results.push(RoomOperationResult::MemberAdded(membership));
                }

                RoomOperation::RemoveMember(room_id, user_id) => {
                    sqlx::query("DELETE FROM room_memberships WHERE room_id = ? AND user_id = ?")
                        .bind(&room_id)
                        .bind(&user_id)
                        .execute(&mut **transaction.inner_mut())
                        .await
                        .map_err(|e| {
                            TransactionError::StorageError(StorageError::QueryError {
                                message: format!("Failed to remove member: {}", e),
                            })
                        })?;

                    results.push(RoomOperationResult::MemberRemoved(room_id, user_id));
                }

                RoomOperation::UpdateMemberRole(room_id, user_id, new_role) => {
                    sqlx::query(
                        "UPDATE room_memberships SET role = ? WHERE room_id = ? AND user_id = ?",
                    )
                    .bind(match new_role {
                        RoomRole::Owner => "owner",
                        RoomRole::Admin => "admin",
                        RoomRole::Member => "member",
                    })
                    .bind(&room_id)
                    .bind(&user_id)
                    .execute(&mut **transaction.inner_mut())
                    .await
                    .map_err(|e| {
                        TransactionError::StorageError(StorageError::QueryError {
                            message: format!("Failed to update member role: {}", e),
                        })
                    })?;

                    // Fetch updated membership
                    let membership_row = sqlx::query(
                        "SELECT * FROM room_memberships WHERE room_id = ? AND user_id = ?",
                    )
                    .bind(&room_id)
                    .bind(&user_id)
                    .fetch_one(&mut **transaction.inner_mut())
                    .await
                    .map_err(|e| {
                        TransactionError::StorageError(StorageError::QueryError {
                            message: format!("Failed to fetch updated membership: {}", e),
                        })
                    })?;

                    let membership = RoomMembership {
                        id: membership_row.get("id"),
                        room_id: membership_row.get("room_id"),
                        user_id: membership_row.get("user_id"),
                        role: new_role,
                        joined_at: membership_row.get::<i64, _>("joined_at") as u64,
                    };

                    results.push(RoomOperationResult::MemberRoleUpdated(membership));
                }
            }
        }

        Ok(results)
    }

    /// Create user with session atomically
    async fn user_registration_transaction(
        &self,
        transaction: &mut Transaction<'_>,
        user: User,
        session: Session,
    ) -> TransactionResult<(User, Session)> {
        transaction.record_operation("user_registration_transaction");

        // Check if username already exists
        let existing_user = sqlx::query("SELECT id FROM users WHERE username = ?")
            .bind(&user.username)
            .fetch_optional(&mut **transaction.inner_mut())
            .await
            .map_err(|e| {
                TransactionError::StorageError(StorageError::QueryError {
                    message: format!("Failed to check username: {}", e),
                })
            })?;

        if existing_user.is_some() {
            return Err(TransactionError::StorageError(
                StorageError::DuplicateError {
                    entity: "User".to_string(),
                    message: "Username already exists".to_string(),
                },
            ));
        }

        // Create user
        sqlx::query(
            "INSERT INTO users (id, username, password_hash, email, created_at, is_active) VALUES (?, ?, ?, ?, ?, ?)"
        )
        .bind(&user.id)
        .bind(&user.username)
        .bind(&user.password_hash)
        .bind(&user.email)
        .bind(user.created_at as i64)
        .bind(user.is_active)
        .execute(&mut **transaction.inner_mut())
        .await
        .map_err(|e| TransactionError::StorageError(StorageError::QueryError {
            message: format!("Failed to create user: {}", e),
        }))?;

        // Create session
        sqlx::query(
            "INSERT INTO sessions (id, user_id, token, created_at, expires_at, is_active) VALUES (?, ?, ?, ?, ?, ?)"
        )
        .bind(&session.id)
        .bind(&session.user_id)
        .bind(&session.token)
        .bind(session.created_at as i64)
        .bind(session.expires_at as i64)
        .bind(session.is_active)
        .execute(&mut **transaction.inner_mut())
        .await
        .map_err(|e| TransactionError::StorageError(StorageError::QueryError {
            message: format!("Failed to create session: {}", e),
        }))?;

        Ok((user, session))
    }

    /// Delete user and cleanup all related data atomically
    async fn user_deletion_transaction(
        &self,
        transaction: &mut Transaction<'_>,
        user_id: &str,
    ) -> TransactionResult<UserDeletionResult> {
        transaction.record_operation("user_deletion_transaction");

        // Count sessions to delete
        let session_count = sqlx::query("SELECT COUNT(*) as count FROM sessions WHERE user_id = ?")
            .bind(user_id)
            .fetch_one(&mut **transaction.inner_mut())
            .await
            .map_err(|e| {
                TransactionError::StorageError(StorageError::QueryError {
                    message: format!("Failed to count sessions: {}", e),
                })
            })?;
        let deleted_sessions = session_count.get::<i64, _>("count") as u64;

        // Count messages to delete
        let message_count =
            sqlx::query("SELECT COUNT(*) as count FROM messages WHERE sender_id = ?")
                .bind(user_id)
                .fetch_one(&mut **transaction.inner_mut())
                .await
                .map_err(|e| {
                    TransactionError::StorageError(StorageError::QueryError {
                        message: format!("Failed to count messages: {}", e),
                    })
                })?;
        let deleted_messages = message_count.get::<i64, _>("count") as u64;

        // Count rooms to be removed from
        let room_count =
            sqlx::query("SELECT COUNT(*) as count FROM room_memberships WHERE user_id = ?")
                .bind(user_id)
                .fetch_one(&mut **transaction.inner_mut())
                .await
                .map_err(|e| {
                    TransactionError::StorageError(StorageError::QueryError {
                        message: format!("Failed to count room memberships: {}", e),
                    })
                })?;
        let removed_from_rooms = room_count.get::<i64, _>("count") as u64;

        // Count invitations to delete
        let invitation_count = sqlx::query(
            "SELECT COUNT(*) as count FROM invitations WHERE recipient_user_id = ? OR sender_user_id = ?",
        )
        .bind(user_id)
        .bind(user_id)
        .fetch_one(&mut **transaction.inner_mut())
        .await
        .map_err(|e| {
            TransactionError::StorageError(StorageError::QueryError {
                message: format!("Failed to count invitations: {}", e),
            })
        })?;
        let deleted_invitations = invitation_count.get::<i64, _>("count") as u64;

        // Delete sessions
        sqlx::query("DELETE FROM sessions WHERE user_id = ?")
            .bind(user_id)
            .execute(&mut **transaction.inner_mut())
            .await
            .map_err(|e| {
                TransactionError::StorageError(StorageError::QueryError {
                    message: format!("Failed to delete sessions: {}", e),
                })
            })?;

        // Delete messages
        sqlx::query("DELETE FROM messages WHERE sender_id = ?")
            .bind(user_id)
            .execute(&mut **transaction.inner_mut())
            .await
            .map_err(|e| {
                TransactionError::StorageError(StorageError::QueryError {
                    message: format!("Failed to delete messages: {}", e),
                })
            })?;

        // Delete room memberships
        sqlx::query("DELETE FROM room_memberships WHERE user_id = ?")
            .bind(user_id)
            .execute(&mut **transaction.inner_mut())
            .await
            .map_err(|e| {
                TransactionError::StorageError(StorageError::QueryError {
                    message: format!("Failed to delete room memberships: {}", e),
                })
            })?;

        // Delete invitations
        sqlx::query("DELETE FROM invitations WHERE recipient_user_id = ? OR sender_user_id = ?")
            .bind(user_id)
            .bind(user_id)
            .execute(&mut **transaction.inner_mut())
            .await
            .map_err(|e| {
                TransactionError::StorageError(StorageError::QueryError {
                    message: format!("Failed to delete invitations: {}", e),
                })
            })?;

        // Delete audit logs
        sqlx::query("DELETE FROM audit_logs WHERE user_id = ?")
            .bind(user_id)
            .execute(&mut **transaction.inner_mut())
            .await
            .map_err(|e| {
                TransactionError::StorageError(StorageError::QueryError {
                    message: format!("Failed to delete audit logs: {}", e),
                })
            })?;

        // Finally, delete the user
        sqlx::query("DELETE FROM users WHERE id = ?")
            .bind(user_id)
            .execute(&mut **transaction.inner_mut())
            .await
            .map_err(|e| {
                TransactionError::StorageError(StorageError::QueryError {
                    message: format!("Failed to delete user: {}", e),
                })
            })?;

        Ok(UserDeletionResult {
            user_id: user_id.to_string(),
            deleted_sessions,
            deleted_messages,
            removed_from_rooms,
            deleted_invitations,
        })
    }

    /// Create room with initial membership atomically
    async fn create_room_with_membership(
        &self,
        transaction: &mut Transaction<'_>,
        room: Room,
        creator_membership: RoomMembership,
    ) -> TransactionResult<(Room, RoomMembership)> {
        transaction.record_operation("create_room_with_membership");

        // Validate that the creator exists
        let user_exists = sqlx::query("SELECT id FROM users WHERE id = ?")
            .bind(&creator_membership.user_id)
            .fetch_optional(&mut **transaction.inner_mut())
            .await
            .map_err(|e| {
                TransactionError::StorageError(StorageError::QueryError {
                    message: format!("Failed to check user existence: {}", e),
                })
            })?;

        if user_exists.is_none() {
            return Err(TransactionError::StorageError(StorageError::NotFound {
                entity: "User".to_string(),
                id: creator_membership.user_id.clone(),
            }));
        }

        // Check if room name already exists (if uniqueness is required)
        let existing_room = sqlx::query("SELECT id FROM rooms WHERE name = ?")
            .bind(&room.name)
            .fetch_optional(&mut **transaction.inner_mut())
            .await
            .map_err(|e| {
                TransactionError::StorageError(StorageError::QueryError {
                    message: format!("Failed to check room name: {}", e),
                })
            })?;

        if existing_room.is_some() {
            return Err(TransactionError::StorageError(
                StorageError::DuplicateError {
                    entity: "Room".to_string(),
                    message: "Room name already exists".to_string(),
                },
            ));
        }

        // Create the room
        sqlx::query(
            "INSERT INTO rooms (id, name, description, is_public, created_at, created_by) VALUES (?, ?, ?, ?, ?, ?)"
        )
        .bind(&room.id)
        .bind(&room.name)
        .bind(&room.description)
        .bind(room.is_public)
        .bind(room.created_at as i64)
        .bind(&room.created_by)
        .execute(&mut **transaction.inner_mut())
        .await
        .map_err(|e| TransactionError::StorageError(StorageError::QueryError {
            message: format!("Failed to create room: {}", e),
        }))?;

        // Create creator membership with owner role
        let owner_membership = RoomMembership {
            role: RoomRole::Owner,
            ..creator_membership
        };

        sqlx::query(
            "INSERT INTO room_memberships (id, room_id, user_id, role, joined_at, status) VALUES (?, ?, ?, ?, ?, ?)"
        )
        .bind(&owner_membership.id)
        .bind(&owner_membership.room_id)
        .bind(&owner_membership.user_id)
        .bind("owner")
        .bind(owner_membership.joined_at as i64)
        .bind("active")
        .execute(&mut **transaction.inner_mut())
        .await
        .map_err(|e| TransactionError::StorageError(StorageError::QueryError {
            message: format!("Failed to create creator membership: {}", e),
        }))?;

        Ok((room, owner_membership))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::storage::transactions::DatabaseTransactionManager;
    use sqlx::SqlitePool;
    use std::sync::Arc;

    async fn create_test_pool() -> sqlx::Pool<Sqlite> {
        let pool = SqlitePool::connect(":memory:").await.unwrap();

        // Create test tables
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS users (
                id TEXT PRIMARY KEY,
                username TEXT UNIQUE NOT NULL,
                password_hash TEXT NOT NULL,
                email TEXT,
                created_at INTEGER NOT NULL,
                is_active BOOLEAN NOT NULL DEFAULT 1
            )",
        )
        .execute(&pool)
        .await
        .unwrap();

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS rooms (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                description TEXT,
                is_public BOOLEAN NOT NULL DEFAULT 0,
                created_at INTEGER NOT NULL,
                created_by TEXT NOT NULL
            )",
        )
        .execute(&pool)
        .await
        .unwrap();

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS room_memberships (
                id TEXT PRIMARY KEY,
                room_id TEXT NOT NULL,
                user_id TEXT NOT NULL,
                role TEXT NOT NULL,
                joined_at INTEGER NOT NULL,
                status TEXT NOT NULL DEFAULT 'active'
            )",
        )
        .execute(&pool)
        .await
        .unwrap();

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS invitations (
                id TEXT PRIMARY KEY,
                room_id TEXT NOT NULL,
                sender_user_id TEXT NOT NULL,
                recipient_user_id TEXT NOT NULL,
                status TEXT NOT NULL DEFAULT 'pending',
                created_at INTEGER NOT NULL,
                expires_at INTEGER NOT NULL
            )",
        )
        .execute(&pool)
        .await
        .unwrap();

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS sessions (
                id TEXT PRIMARY KEY,
                user_id TEXT NOT NULL,
                token TEXT NOT NULL,
                created_at INTEGER NOT NULL,
                expires_at INTEGER NOT NULL,
                is_active BOOLEAN NOT NULL DEFAULT 1
            )",
        )
        .execute(&pool)
        .await
        .unwrap();

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS messages (
                id TEXT PRIMARY KEY,
                room_id TEXT NOT NULL,
                sender_id TEXT NOT NULL,
                content TEXT NOT NULL,
                created_at INTEGER NOT NULL,
                message_type TEXT NOT NULL DEFAULT 'text'
            )",
        )
        .execute(&pool)
        .await
        .unwrap();

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS audit_logs (
                id TEXT PRIMARY KEY,
                user_id TEXT,
                action TEXT NOT NULL,
                details TEXT,
                created_at INTEGER NOT NULL
            )",
        )
        .execute(&pool)
        .await
        .unwrap();

        pool
    }

    async fn create_test_user(pool: &sqlx::Pool<Sqlite>) -> User {
        let user = User {
            id: generate_id(),
            username: "testuser".to_string(),
            password_hash: "hash123".to_string(),
            email: Some("test@example.com".to_string()),
            created_at: current_timestamp(),
            is_active: true,
        };

        sqlx::query(
            "INSERT INTO users (id, username, password_hash, email, created_at, is_active) VALUES (?, ?, ?, ?, ?, ?)"
        )
        .bind(&user.id)
        .bind(&user.username)
        .bind(&user.password_hash)
        .bind(&user.email)
        .bind(user.created_at as i64)
        .bind(user.is_active)
        .execute(pool)
        .await
        .unwrap();

        user
    }

    async fn create_test_room(pool: &sqlx::Pool<Sqlite>, creator_id: &str) -> Room {
        let room = Room {
            id: generate_id(),
            name: "Test Room".to_string(),
            description: Some("A test room".to_string()),
            is_public: true,
            created_at: current_timestamp(),
            created_by: creator_id.to_string(),
        };

        sqlx::query(
            "INSERT INTO rooms (id, name, description, is_public, created_at, created_by) VALUES (?, ?, ?, ?, ?, ?)"
        )
        .bind(&room.id)
        .bind(&room.name)
        .bind(&room.description)
        .bind(room.is_public)
        .bind(room.created_at as i64)
        .bind(&room.created_by)
        .execute(pool)
        .await
        .unwrap();

        room
    }

    #[tokio::test]
    async fn test_create_invitation_with_membership() {
        let pool = Arc::new(create_test_pool().await);
        let manager = DatabaseTransactionManager::with_defaults(pool.clone());
        let operations = AtomicOperations::new();

        // Create test users
        let inviter = create_test_user(&pool).await;
        let invitee = create_test_user(&pool).await;

        // Create test room
        let room = create_test_room(&pool, &inviter.id).await;

        // Create inviter membership
        let inviter_membership = RoomMembership {
            id: generate_id(),
            room_id: room.id.clone(),
            user_id: inviter.id.clone(),
            role: RoomRole::Admin,
            joined_at: current_timestamp(),
        };

        sqlx::query(
            "INSERT INTO room_memberships (id, room_id, user_id, role, joined_at, status) VALUES (?, ?, ?, ?, ?, ?)"
        )
        .bind(&inviter_membership.id)
        .bind(&inviter_membership.room_id)
        .bind(&inviter_membership.user_id)
        .bind("admin")
        .bind(inviter_membership.joined_at as i64)
        .bind("active")
        .execute(&pool)
        .await
        .unwrap();

        // Create invitation and membership
        let invitation = Invitation {
            id: generate_id(),
            room_id: room.id.clone(),
            sender_user_id: inviter.id.clone(),
            recipient_user_id: invitee.id.clone(),
            invitation_type: InvitationType::RoomInvitation,
            status: InvitationStatus::Pending,
            message: None,
            created_at: current_timestamp(),
            expires_at: Some(current_timestamp() + 3600), // 1 hour from now
            responded_at: None,
            metadata: InvitationMetadata {
                sender_permissions: Vec::new(),
                recipient_permissions: Vec::new(),
                invitation_context: None,
                custom_fields: std::collections::HashMap::new(),
            },
        };

        let membership = RoomMembership {
            id: generate_id(),
            room_id: room.id.clone(),
            user_id: invitee.id.clone(),
            role: RoomRole::Member,
            joined_at: current_timestamp(),
        };

        let mut transaction = manager.begin_transaction().await.unwrap();
        let result = operations
            .create_invitation_with_membership(
                &mut transaction,
                invitation.clone(),
                membership.clone(),
            )
            .await;

        assert!(result.is_ok());
        manager.commit_transaction(transaction).await.unwrap();

        // Verify invitation was created
        let stored_invitation = sqlx::query("SELECT * FROM invitations WHERE id = ?")
            .bind(&invitation.id)
            .fetch_one(&pool)
            .await
            .unwrap();

        assert_eq!(stored_invitation.get::<String, _>("status"), "pending");
    }

    #[tokio::test]
    async fn test_user_registration_transaction() {
        let pool = Arc::new(create_test_pool().await);
        let manager = DatabaseTransactionManager::with_defaults(pool.clone());
        let operations = AtomicOperations::new();

        let user = User {
            id: generate_id(),
            username: "newuser".to_string(),
            password_hash: "hash123".to_string(),
            email: Some("newuser@example.com".to_string()),
            created_at: current_timestamp(),
            is_active: true,
        };

        let session = Session {
            id: generate_id(),
            user_id: user.id.clone(),
            token: "token123".to_string(),
            created_at: current_timestamp(),
            expires_at: current_timestamp() + 3600,
            is_active: true,
        };

        let mut transaction = manager.begin_transaction().await.unwrap();
        let result = operations
            .user_registration_transaction(&mut transaction, user.clone(), session.clone())
            .await;

        assert!(result.is_ok());
        manager.commit_transaction(transaction).await.unwrap();

        // Verify user was created
        let stored_user = sqlx::query("SELECT * FROM users WHERE id = ?")
            .bind(&user.id)
            .fetch_one(&pool)
            .await
            .unwrap();

        assert_eq!(stored_user.get::<String, _>("username"), user.username);

        // Verify session was created
        let stored_session = sqlx::query("SELECT * FROM sessions WHERE id = ?")
            .bind(&session.id)
            .fetch_one(&pool)
            .await
            .unwrap();

        assert_eq!(stored_session.get::<String, _>("token"), session.token);
    }

    #[tokio::test]
    async fn test_create_room_with_membership() {
        let pool = Arc::new(create_test_pool().await);
        let manager = DatabaseTransactionManager::with_defaults(pool.clone());
        let operations = AtomicOperations::new();

        // Create test user
        let creator = create_test_user(&pool).await;

        let room = Room {
            id: generate_id(),
            name: "New Room".to_string(),
            description: Some("A new room".to_string()),
            is_public: false,
            created_at: current_timestamp(),
            created_by: creator.id.clone(),
        };

        let membership = RoomMembership {
            id: generate_id(),
            room_id: room.id.clone(),
            user_id: creator.id.clone(),
            role: RoomRole::Member, // Will be changed to Owner
            joined_at: current_timestamp(),
        };

        let mut transaction = manager.begin_transaction().await.unwrap();
        let result = operations
            .create_room_with_membership(&mut transaction, room.clone(), membership.clone())
            .await;

        assert!(result.is_ok());
        let (_, created_membership) = result.unwrap();
        assert_eq!(created_membership.role, RoomRole::Owner);

        manager.commit_transaction(transaction).await.unwrap();

        // Verify room was created
        let stored_room = sqlx::query("SELECT * FROM rooms WHERE id = ?")
            .bind(&room.id)
            .fetch_one(&pool)
            .await
            .unwrap();

        assert_eq!(stored_room.get::<String, _>("name"), room.name);

        // Verify membership was created with owner role
        let stored_membership =
            sqlx::query("SELECT * FROM room_memberships WHERE room_id = ? AND user_id = ?")
                .bind(&room.id)
                .bind(&creator.id)
                .fetch_one(&pool)
                .await
                .unwrap();

        assert_eq!(stored_membership.get::<String, _>("role"), "owner");
    }
}
