"""Record the validated gateway configuration when Hermes creates a session.

This is configured-provider provenance, not a billing-provider or cost claim.
Actual upstream route evidence is independently enforced by observed_send.
"""


def install_session_provenance(session_class, profile):
    original = session_class.create_session
    provider = profile["model"]["provider"]
    selected_model = profile["model"]["default"]

    def create_session(db, session_id, source, model=None, model_config=None, **kwargs):
        if model != selected_model:
            raise RuntimeError("session model differs from the captured profile")
        config = dict(model_config or {})
        if config.get("provider", provider) != provider:
            raise RuntimeError("session provider differs from the captured profile")
        config["provider"] = provider
        return original(db, session_id=session_id, source=source, model=model,
                        model_config=config, **kwargs)

    session_class.create_session = create_session
