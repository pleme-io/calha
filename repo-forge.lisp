(defrepo calha
  :description "Calha (Brazilian-Portuguese, gutter/channel) -- a Kubernetes controller that drives the restart-required half of a config-hot-swap change through a normal rolling update, gated by formigueiro's outorga shadow-first promotion policy, structurally walled to the DISCOVERED config tier (never touches git/Flux-owned config). Composes shikumi::ConfigStore/resolve_progressive/Provenance + breathe-provider's ConfigReload/DisruptionClass shape + formigueiro::outorga::PromotionPolicy + engenho_controllers::Controller. Design tier per theory/CALHA.md; this repo carries the M2-M4 controller work (CalhaPolicy CRD, plan_tick, the controller binary)."
  :kind :rust-tool
  :visibility :public
  :binary "calha"
  :external-crates
    ((:name "tokio"              :version "1"     :features ("full"))
     (:name "serde"              :version "1"     :features ("derive"))
     (:name "serde_json"         :version "1")
     (:name "serde_yaml_ng"      :version "0.10")
     (:name "thiserror"          :version "2")
     (:name "anyhow"             :version "1")
     (:name "async-trait"        :version "0.1")
     (:name "clap"               :version "4"     :features ("derive" "env"))
     (:name "tracing"            :version "0.1")
     (:name "tracing-subscriber" :version "0.3"   :features ("env-filter" "fmt" "json"))
     (:name "schemars"           :version "0.8")
     (:name "chrono"             :version "0.4"   :features ("serde"))
     (:name "kube"               :version "0.99"  :features ("client" "rustls-tls" "runtime" "derive"))
     (:name "k8s-openapi"        :version "0.24"  :features ("latest"))
     (:name "reqwest"            :version "0.12"  :features ("json" "rustls-tls")))
  :ci
    (:systems       ("aarch64-darwin" "x86_64-linux" "aarch64-linux")
     :test-systems  ("aarch64-darwin")
     :build-images  #f))
