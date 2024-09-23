use std::fmt::Write;

/// Generates twirp services for protobuf rpc service definitions.
///
/// In your `build.rs`, using `prost_build`, you can wire in the twirp
/// `ServiceGenerator` to produce a Rust server for your proto services.
///
/// Add a call to `.service_generator(twirp_build::service_generator())` in
/// main() of `build.rs`.
pub fn service_generator() -> Box<ServiceGenerator> {
    Box::new(ServiceGenerator {
        async_trait_shim: true,
    })
}

pub struct ServiceGenerator {
    /// Whether the Service Generator should add the `#[async_trait::async_trait]` 
    /// attribute to generated traits and implementations. Useful for supporting 
    /// rust versions prior to 1.75 which do not support async fns in traits.
    ///
    /// This value is set to true by default.
    async_trait_shim: bool,
}

impl ServiceGenerator {
    /// Whether the Service Generator should add the `#[async_trait::async_trait]` 
    /// attribute to generated traits and implementations. Useful for supporting 
    /// rust versions prior to 1.75 which do not support async fns in traits.
    ///
    /// This value is set to true by default.
    pub fn async_trait_shim(mut self, async_trait_shim: bool) -> ServiceGenerator {
        self.async_trait_shim = async_trait_shim;
        self
    }
}

impl prost_build::ServiceGenerator for ServiceGenerator {
    fn generate(&mut self, service: prost_build::Service, buf: &mut String) {
        let service_name = service.name;
        let service_fqn = format!("{}.{}", service.package, service.proto_name);
        writeln!(buf).unwrap();

        writeln!(buf, "pub use twirp;").unwrap();
        writeln!(buf).unwrap();
        writeln!(buf, "pub const SERVICE_FQN: &str = \"/{service_fqn}\";").unwrap();

        //
        // generate the twirp server
        //
        if self.async_trait_shim {
            writeln!(buf, "#[twirp::async_trait::async_trait]").unwrap();
        }
        writeln!(buf, "pub trait {} {{", service_name).unwrap();
        for m in &service.methods {
            writeln!(
                buf,
                "    async fn {}(&self, ctx: twirp::Context, req: {}) -> Result<{}, twirp::TwirpErrorResponse>;",
                m.name, m.input_type, m.output_type,
            )
            .unwrap();
        }
        writeln!(buf, "}}").unwrap();

        if self.async_trait_shim {
            writeln!(buf, "#[twirp::async_trait::async_trait]").unwrap();
        }
        writeln!(buf, "impl<T> {service_name} for std::sync::Arc<T>").unwrap();
        writeln!(buf, "where").unwrap();
        writeln!(buf, "    T: {service_name} + Sync + Send").unwrap();
        writeln!(buf, "{{").unwrap();
        for m in &service.methods {
            writeln!(
                buf,
                "    async fn {}(&self, ctx: twirp::Context, req: {}) -> Result<{}, twirp::TwirpErrorResponse> {{",
                m.name, m.input_type, m.output_type,
            )
                .unwrap();
            writeln!(buf, "        (*self).{}(ctx, req).await", m.name).unwrap();
            writeln!(buf, "    }}").unwrap();
        }
        writeln!(buf, "}}").unwrap();

        // add_service
        writeln!(
            buf,
            r#"pub fn router<T>(api: T) -> twirp::Router
where
    T: {service_name} + Clone + Send + Sync + 'static,
{{
    twirp::details::TwirpRouterBuilder::new(api)"#,
        )
        .unwrap();
        for m in &service.methods {
            let uri = &m.proto_name;
            let req_type = &m.input_type;
            let rust_method_name = &m.name;
            writeln!(
                buf,
                r#"        .route("/{uri}", |api: T, ctx: twirp::Context, req: {req_type}| async move {{
            api.{rust_method_name}(ctx, req).await
        }})"#,
            )
            .unwrap();
        }
        writeln!(
            buf,
            r#"
        .build()
}}"#
        )
        .unwrap();

        //
        // generate the twirp client
        //
        writeln!(buf).unwrap();
        if self.async_trait_shim {
            writeln!(buf, "#[twirp::async_trait::async_trait]").unwrap();
        }
        writeln!(
            buf,
            "pub trait {service_name}Client: Send + Sync + std::fmt::Debug {{",
        )
        .unwrap();
        for m in &service.methods {
            // Define: <METHOD>
            writeln!(
                buf,
                "    async fn {}(&self, req: {}) -> Result<{}, twirp::ClientError>;",
                m.name, m.input_type, m.output_type,
            )
            .unwrap();
        }
        writeln!(buf, "}}").unwrap();

        // Implement the rpc traits for: `twirp::client::Client`
        if self.async_trait_shim {
            writeln!(buf, "#[twirp::async_trait::async_trait]").unwrap();
        }
        writeln!(
            buf,
            "impl {service_name}Client for twirp::client::Client {{",
        )
        .unwrap();
        for m in &service.methods {
            // Define the rpc `<METHOD>`
            writeln!(
                buf,
                "    async fn {}(&self, req: {}) -> Result<{}, twirp::ClientError> {{",
                m.name, m.input_type, m.output_type,
            )
            .unwrap();
            writeln!(
                buf,
                r#"    self.request("{}/{}", req).await"#,
                service_fqn, m.proto_name
            )
            .unwrap();
            writeln!(buf, "    }}").unwrap();
        }
        writeln!(buf, "}}").unwrap();
    }
}
