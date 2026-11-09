# MMD-10 variants (QA test input, derived from test_mermaid_issue_165.md FC-165b)

## V0 baseline (autonumber)

```mermaid
sequenceDiagram
    autonumber
    participant PEP as 业务PEP客户端(网关/堡垒机/AI Agent)
    participant AUTH as 认证服务模块
    participant CACHE as 本地缓存/Redis缓存
    participant PDP as OPA授权决策模块

    PEP->>AUTH: 携带Authorization Bearer Token发起鉴权请求
    AUTH->>AUTH: 校验请求头格式、参数合法性
    AUTH->>CACHE: 查询令牌黑名单/缓存会话信息
    CACHE-->>AUTH: 返回缓存校验结果
    AUTH->>AUTH: JWT签名校验、过期时间校验、签发方校验
    AUTH->>AUTH: 解析标准化身份属性(租户/应用/用户/角色/属性)
    AUTH->>AUTH: 校验租户、应用归属合法性
    alt 认证失败
        AUTH-->>PEP: 返回认证失败(错误码+原因)，终止流程
    else 认证成功
        AUTH-->>PDP: 推送标准化可信身份结构体
        PDP->>PDP: 进入策略授权决策流程
    end
```

## VA: autonumber off after 4th message (expect badges 1-4 only)

```mermaid
sequenceDiagram
    autonumber
    participant PEP as 业务PEP客户端(网关/堡垒机/AI Agent)
    participant AUTH as 认证服务模块
    participant CACHE as 本地缓存/Redis缓存
    participant PDP as OPA授权决策模块

    PEP->>AUTH: 携带Authorization Bearer Token发起鉴权请求
    AUTH->>AUTH: 校验请求头格式、参数合法性
    AUTH->>CACHE: 查询令牌黑名单/缓存会话信息
    CACHE-->>AUTH: 返回缓存校验结果
    autonumber off
    AUTH->>AUTH: JWT签名校验、过期时间校验、签发方校验
    AUTH->>AUTH: 解析标准化身份属性(租户/应用/用户/角色/属性)
    AUTH->>AUTH: 校验租户、应用归属合法性
    alt 认证失败
        AUTH-->>PEP: 返回认证失败(错误码+原因)，终止流程
    else 认证成功
        AUTH-->>PDP: 推送标准化可信身份结构体
        PDP->>PDP: 进入策略授权决策流程
    end
```

## VB: autonumber 10 10 (expect 10,20,30...)

```mermaid
sequenceDiagram
    autonumber 10 10
    participant PEP as 业务PEP客户端(网关/堡垒机/AI Agent)
    participant AUTH as 认证服务模块
    participant CACHE as 本地缓存/Redis缓存
    participant PDP as OPA授权决策模块

    PEP->>AUTH: 携带Authorization Bearer Token发起鉴权请求
    AUTH->>AUTH: 校验请求头格式、参数合法性
    AUTH->>CACHE: 查询令牌黑名单/缓存会话信息
    CACHE-->>AUTH: 返回缓存校验结果
    AUTH->>AUTH: JWT签名校验、过期时间校验、签发方校验
    AUTH->>AUTH: 解析标准化身份属性(租户/应用/用户/角色/属性)
    AUTH->>AUTH: 校验租户、应用归属合法性
    alt 认证失败
        AUTH-->>PEP: 返回认证失败(错误码+原因)，终止流程
    else 认证成功
        AUTH-->>PDP: 推送标准化可信身份结构体
        PDP->>PDP: 进入策略授权决策流程
    end
```
