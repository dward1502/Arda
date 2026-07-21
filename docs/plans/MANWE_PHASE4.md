MANWE ADAPTIVE ROUTING — PHASE 4 PLAN                              
     =====================================                              
                                                                        
     Reference: docs/CHATGPT_MANWE.md                                   
     Target crate: crates/spine/runtime/manwe (feature flag:            
     adaptive)                                                          
                                                                        
     Top-level constraint                                               
     - Default manwe stays the stable OpenAI-compatible gateway         
     - Adaptive tree lives under src/adaptive/                          
     - First compile and test in-memory with mock adapters only         
     - No Arda workspace-wide breakage; default feature remains         
     compileable                                                        
                                                                        
     Step 1 — Adaptive types and error model                            
     - Add src/adaptive/types.rs for route/history identifiers and      
     model/provider IDs                                                 
     - Add src/adaptive/error.rs for decision-only failures             
     - Keep the stable gateway error model separate                     
                                                                        
     Step 2 — Provider capabilities and health                          
     - Add src/adaptive/provider.rs                                     
     - Define capability tags, rate limits, token caps, supported       
     features                                                           
     - Add health probe state: unknown, probing, healthy, degraded,     
     down                                                               
                                                                        
     Step 3 — Immutable route candidates                                
     - Add src/adaptive/candidate.rs                                    
     - Model concrete provider+model pairings with derived              
     capabilities                                                       
     - Make candidates cheap to compare and sortable                    
                                                                        
     Step 4 — Route policy                                              
     - Add src/adaptive/policy.rs                                       
     - Policy is a typed struct: required capabilities, allowlist,      
     blocklist, minimum health state                                    
     - Validation should fail fast at parse/bind time                   
                                                                        
     Step 5 — Deterministic scoring                                     
     - Add src/adaptive/score.rs                                        
     - Include health weight, latency estimate, quota slack, cost,      
     bandit prior, policy match                                         
     - Must be pure and deterministic given identical inputs            
                                                                        
     Step 6 — Deterministic selection                                   
     - Add src/adaptive/selector.rs                                     
     - Sort eligible candidates by score, then stable tiebreaker        
     - Never decide under shared mutability                             
                                                                        
     Step 7 — Fallback behavior                                         
     - Add src/adaptive/fallback.rs                                     
     - Define max fallback attempts per request                         
     - Stop on provider/auth errors vs retry on                         
     transient/unavailable errors                                       
                                                                        
     Step 8 — Sessions and history                                      
     - Add src/adaptive/session.rs                                      
     - Track recent providers per session key, last successes, last     
     failure bucket                                                     
     - Keep bounded history to avoid unbounded memory growth            
                                                                        
     Step 9 — Quotas                                                    
     - Add src/adaptive/quota.rs                                        
     - Scope by provider, model, and session/user facet                 
     - Track used/total/last reset                                      
                                                                        
     Step 10 — Bandit learning                                          
     - Add src/adaptive/bandit.rs                                       
     - Start with per-provider-model success/failure counters           
     - Apply epsilon-greedy discovery with configurable epsilon and     
     minimum observation threshold                                      
                                                                        
     Step 11 — Persisted state                                          
     - Add src/adaptive/state.rs                                        
     - In-memory snapshot: bandit state, quota counts, last probes,     
     candidate cache                                                    
     - Add optional periodic persistence hooks behind an in-memory      
     store trait                                                        
                                                                        
     Step 12 — Governance/economics adapters                            
     - Add src/adaptive/adapters.rs                                     
     - Define traits for governance, economics, treaty/budget           
     checks                                                             
     - Provide mock adapters for compilation and tests                  
                                                                        
     Step 13 — Administrative transports                                
     - Add src/adaptive/admin.rs                                        
     - Internal admin surface to report mode, capabilities, quotas,     
     provider health                                                    
     - Do not expose real auth/admin externally yet                     
                                                                        
     Step 14 — External drivers                                         
     - Add src/adaptive/drivers.rs                                      
     - Wire to runtime probes, config reload, route history hooks,      
     and gateway mode reporting                                         
     - Keep gateway-level behavior unchanged unless adaptive is         
     enabled                                                            
                                                                        
     Implementation guardrails                                          
     - src/adaptive/mod.rs re-exports only                              
     - Everything behind #[cfg(feature = "adaptive")]                   
     - Stable root never depends on adaptive internals                  
     - Add a manwe --capabilities style mode report so downstream       
     knows static vs adaptive                                           
                                                                        
     Suggested acceptance criteria                                      
     - cargo check -p manwe                                             
     - cargo check -p manwe --features adaptive                         
     - cargo test -p manwe --features adaptive with mock adapters       
     only                                                               
                                       