use super::*;

impl BslLanguageServer {
    pub(crate) async fn sync_formatting_capability_registration(&self) {
        const DOC_FORMATTING_ID: &str = "bsl.formatting";
        const RANGE_FORMATTING_ID: &str = "bsl.rangeFormatting";

        let enabled = self.settings.read().await.formatting.enabled;

        let (spawn_worker, dynamic_doc, dynamic_range) = {
            let mut state = self.formatting_capability.write().await;
            state.desired_enabled = enabled;

            if !(state.dynamic_document_formatting || state.dynamic_range_formatting) {
                return;
            }

            if state.in_flight {
                return;
            }

            if state.registered == state.desired_enabled {
                return;
            }

            state.in_flight = true;
            (
                true,
                state.dynamic_document_formatting,
                state.dynamic_range_formatting,
            )
        };

        if !spawn_worker {
            return;
        }

        let client = self.client.clone();
        let state = self.formatting_capability.clone();

        tokio::spawn(async move {
            loop {
                let (desired_enabled, currently_registered) = {
                    let guard = state.read().await;
                    (guard.desired_enabled, guard.registered)
                };

                if desired_enabled == currently_registered {
                    let mut guard = state.write().await;
                    guard.in_flight = false;
                    return;
                }

                let result = if desired_enabled {
                    let mut registrations = Vec::new();
                    if dynamic_doc {
                        registrations.push(Registration {
                            id: DOC_FORMATTING_ID.to_string(),
                            method: DocumentFormattingRequest::METHOD.to_string(),
                            register_options: Some(serde_json::json!({ "documentSelector": null })),
                        });
                    }
                    if dynamic_range {
                        registrations.push(Registration {
                            id: RANGE_FORMATTING_ID.to_string(),
                            method: RangeFormatting::METHOD.to_string(),
                            register_options: Some(serde_json::json!({ "documentSelector": null })),
                        });
                    }

                    client.register_capability(registrations).await
                } else {
                    let mut unregisterations = Vec::new();
                    if dynamic_doc {
                        unregisterations.push(Unregistration {
                            id: DOC_FORMATTING_ID.to_string(),
                            method: DocumentFormattingRequest::METHOD.to_string(),
                        });
                    }
                    if dynamic_range {
                        unregisterations.push(Unregistration {
                            id: RANGE_FORMATTING_ID.to_string(),
                            method: RangeFormatting::METHOD.to_string(),
                        });
                    }

                    client.unregister_capability(unregisterations).await
                };

                match result {
                    Ok(()) => {
                        let mut guard = state.write().await;
                        guard.registered = desired_enabled;
                    }
                    Err(err) => {
                        warn!(
                            "Failed to {} formatting capability: {}",
                            if desired_enabled {
                                "register"
                            } else {
                                "unregister"
                            },
                            err
                        );
                        let mut guard = state.write().await;
                        guard.in_flight = false;
                        return;
                    }
                }
            }
        });
    }

    pub(crate) async fn sync_inlay_hints_capability_registration(&self) {
        const INLAY_HINTS_ID: &str = "bsl.inlayHints";

        let enabled = {
            let settings_enabled = self.settings.read().await.type_hints.enabled;
            let gate_enabled = self
                .config
                .read()
                .await
                .as_ref()
                .and_then(|cfg| cfg.enable_type_hints)
                .unwrap_or(false);
            settings_enabled && gate_enabled
        };

        let spawn_worker = {
            let mut state = self.inlay_hints_capability.write().await;
            state.desired_enabled = enabled;

            if !state.dynamic_registration {
                return;
            }

            if state.in_flight {
                return;
            }

            if state.registered == state.desired_enabled {
                return;
            }

            state.in_flight = true;
            true
        };

        if !spawn_worker {
            return;
        }

        let client = self.client.clone();
        let state = self.inlay_hints_capability.clone();

        tokio::spawn(async move {
            loop {
                let (desired_enabled, currently_registered) = {
                    let guard = state.read().await;
                    (guard.desired_enabled, guard.registered)
                };

                if desired_enabled == currently_registered {
                    let mut guard = state.write().await;
                    guard.in_flight = false;
                    return;
                }

                let result = if desired_enabled {
                    client
                        .register_capability(vec![Registration {
                            id: INLAY_HINTS_ID.to_string(),
                            method: InlayHintRequest::METHOD.to_string(),
                            register_options: Some(serde_json::json!({ "documentSelector": null })),
                        }])
                        .await
                } else {
                    client
                        .unregister_capability(vec![Unregistration {
                            id: INLAY_HINTS_ID.to_string(),
                            method: InlayHintRequest::METHOD.to_string(),
                        }])
                        .await
                };

                match result {
                    Ok(()) => {
                        let mut guard = state.write().await;
                        guard.registered = desired_enabled;
                    }
                    Err(err) => {
                        warn!(
                            "Failed to {} inlay hints capability: {}",
                            if desired_enabled {
                                "register"
                            } else {
                                "unregister"
                            },
                            err
                        );
                        let mut guard = state.write().await;
                        guard.in_flight = false;
                        return;
                    }
                }
            }
        });
    }

    pub(crate) async fn sync_code_actions_capability_registration(&self) {
        const CODE_ACTIONS_ID: &str = "bsl.codeActions";

        let enabled = {
            let settings_enabled = self.settings.read().await.code_actions.enabled;
            let gate_enabled = self
                .config
                .read()
                .await
                .as_ref()
                .and_then(|cfg| cfg.enable_code_actions)
                .unwrap_or(false);
            settings_enabled && gate_enabled
        };

        let spawn_worker = {
            let mut state = self.code_actions_capability.write().await;
            state.desired_enabled = enabled;

            if !state.dynamic_registration {
                return;
            }

            if state.in_flight {
                return;
            }

            if state.registered == state.desired_enabled {
                return;
            }

            state.in_flight = true;
            true
        };

        if !spawn_worker {
            return;
        }

        let client = self.client.clone();
        let state = self.code_actions_capability.clone();

        tokio::spawn(async move {
            loop {
                let (desired_enabled, currently_registered) = {
                    let guard = state.read().await;
                    (guard.desired_enabled, guard.registered)
                };

                if desired_enabled == currently_registered {
                    let mut guard = state.write().await;
                    guard.in_flight = false;
                    return;
                }

                let result = if desired_enabled {
                    client
                        .register_capability(vec![Registration {
                            id: CODE_ACTIONS_ID.to_string(),
                            method: CodeActionRequest::METHOD.to_string(),
                            register_options: Some(serde_json::json!({
                                "documentSelector": null,
                                "codeActionKinds": ["quickfix", "refactor.extract"]
                            })),
                        }])
                        .await
                } else {
                    client
                        .unregister_capability(vec![Unregistration {
                            id: CODE_ACTIONS_ID.to_string(),
                            method: CodeActionRequest::METHOD.to_string(),
                        }])
                        .await
                };

                match result {
                    Ok(()) => {
                        let mut guard = state.write().await;
                        guard.registered = desired_enabled;
                    }
                    Err(err) => {
                        warn!(
                            "Failed to {} code actions capability: {}",
                            if desired_enabled {
                                "register"
                            } else {
                                "unregister"
                            },
                            err
                        );
                        let mut guard = state.write().await;
                        guard.in_flight = false;
                        return;
                    }
                }
            }
        });
    }
}
