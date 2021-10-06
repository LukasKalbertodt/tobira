import { Translation } from "react-i18next";
import React, { ReactNode } from "react";

import { APIError, ErrorKind, NetworkError, NotJson, ServerError } from ".";
import { Root } from "../layout/Root";
import { RoutingContext } from "../router";
import { Card } from "../ui/Card";
import { assertNever, bug } from "../util/err";
import { match } from "../util";
import { TFunction } from "i18next";


type Props = {
    children: ReactNode;
};

type HandledError = NetworkError | ServerError | APIError | NotJson;

type State = {
    error: null | HandledError;
};

export class GraphQLErrorBoundary extends React.Component<Props, State> {
    public declare context: React.ContextType<typeof RoutingContext>;
    public static contextType = RoutingContext;

    public constructor(props: Props, context: React.ContextType<typeof RoutingContext>) {
        super(props, context);

        const initialState = { error: null };
        this.state = initialState;

        // Reset this state whenever the route changes.
        if (this.context === null) {
            return bug("API error boundary not child of router!");
        }
        this.context.listen(() => this.setState(initialState));
    }

    public static getDerivedStateFromError(error: unknown): State {
        if (error instanceof NetworkError
            || error instanceof ServerError
            || error instanceof NotJson
            || error instanceof APIError) {
            return { error };
        }

        // Not our problem
        return { error: null };
    }

    public render(): ReactNode {
        const error = this.state.error;
        if (error === null) {
            return this.props.children;
        }

        return (
            <Root nav={[]}>
                <Translation>{t => (
                    <div css={{ margin: "0 auto", maxWidth: 600 }}>
                        <MainErrorMessage t={t} error={error} />
                        <p css={{ marginBottom: 16, marginTop: "min(150px, 12vh)" }}>
                            {t("api.error-boundary.detailed-error-info")}
                        </p>
                        <div css={{
                            backgroundColor: "var(--grey97)",
                            borderRadius: 4,
                            padding: 16,
                            fontSize: 14,
                        }}>
                            <pre><code>{error.toString()}</code></pre>
                        </div>
                    </div>
                )}</Translation>
            </Root>
        );
    }
}

type MainErrorMessageProps = {
    error: HandledError;
    t: TFunction;
};

const MainErrorMessage: React.FC<MainErrorMessageProps> = ({ error, t }) => {
    let message: string | JSX.Element;
    let ourFault = false;
    if (error instanceof NetworkError) {
        message = t("errors.network-error");
    } else if (error instanceof ServerError) {
        // TODO: choose better error messages according to status code
        message = t("api.error-boundary.unexpected-server-error");
        ourFault = true;
    } else if (error instanceof NotJson) {
        message = t("errors.unexpected-response");
        ourFault = true;
    } else if (error instanceof APIError) {
        // OK response, but it contained GraphQL errors.

        const kindToMessage = (kind: ErrorKind | undefined) => {
            if (!kind) {
                ourFault = true;
                return t("api.error-boundary.unexpected-server-error");
            } else {
                return match(kind, {
                    INTERNAL_SERVER_ERROR: () => {
                        ourFault = true;
                        return t("errors.internal-server-error");
                    },
                    INVALID_INPUT: () => t("api.error-boundary.invalid-input"),
                    NOT_AUTHORIZED: () => t("errors.not-authorized"),
                });
            }
        };

        const kinds = new Set(error.errors.map(e => e.kind));
        console.log(error.errors);
        console.log(kinds);
        if (kinds.size === 0) {
            // This should never happen?
            message = t("api.error-boundary.unexpected-server-error");
            ourFault = true;
        } else if (kinds.size === 1) {
            message = kindToMessage(Array.from(kinds)[0]);
        } else {
            message = <ul>
                {Array.from(kinds).map(kind => <li key={kind}>{kindToMessage(kind)}</li>)}
            </ul>;
        }
    } else {
        // Typescript unfortunately requires this `else` branch for some reason.
        message = assertNever(error);
    }

    return <>
        <div css={{ textAlign: "center" }}>
            <Card kind="error">{message}</Card>
        </div>
        {ourFault && <p css={{ margin: "24px 0" }}>
            {t("api.error-boundary.not-your-fault")}
        </p>}
    </>;
};
