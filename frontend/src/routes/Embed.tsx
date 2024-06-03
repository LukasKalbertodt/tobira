import { ReactNode, Suspense } from "react";
import { LuFrown, LuAlertTriangle } from "react-icons/lu";
import { Translation, useTranslation } from "react-i18next";
import { graphql, PreloadedQuery, usePreloadedQuery } from "react-relay";
import { unreachable } from "@opencast/appkit";

import { eventId, isSynced, keyOfId } from "../util";
import { GlobalErrorBoundary } from "../util/err";
import { loadQuery } from "../relay";
import { makeRoute } from "../rauta";
import { Player, PlayerPlaceholder } from "../ui/player";
import { Spinner } from "../ui/Spinner";
import { MovingTruck } from "../ui/Waiting";
import { b64regex } from "./util";
import { EmbedQuery } from "./__generated__/EmbedQuery.graphql";
import { PlayerContextProvider } from "../ui/player/PlayerContext";


const query = graphql`
    query EmbedQuery($id: ID!) {
        eventById(id: $id) {
            __typename
            ... on NotAllowed { dummy }
            ... on AuthorizedEvent {
                title
                created
                isLive
                opencastId
                creators
                metadata
                description
                series { title opencastId }
                syncedData {
                    updated
                    startTime
                    endTime
                    duration
                    thumbnail
                    tracks { uri flavor mimetype resolution isMaster }
                    captions { uri lang }
                    segments { uri startTime }
                }
            }
        }
    }
`;

export const EmbedVideoRoute = makeRoute({
    url: ({ videoId }: { videoId: string }) => `/~embed/!v/${keyOfId(videoId)}`,
    match: url => {
        const regex = new RegExp(`^/~embed/!v/(${b64regex}+)/?$`, "u");
        const params = regex.exec(url.pathname);
        if (params === null) {
            return null;
        }
        const videoId = decodeURIComponent(params[1]);

        const queryRef = loadQuery<EmbedQuery>(query, { id: eventId(videoId) });

        return {
            render: () => <ErrorBoundary>
                <Suspense fallback={
                    <PlayerPlaceholder>
                        <Spinner css={{
                            "& > circle": {
                                stroke: "white",
                            },
                        }} />
                    </PlayerPlaceholder>
                }>
                    <PlayerContextProvider>
                        <Embed queryRef={queryRef} />
                    </PlayerContextProvider>
                </Suspense>
            </ErrorBoundary>,
            dispose: () => queryRef.dispose(),
        };
    },
});

type EmbedProps = {
    queryRef: PreloadedQuery<EmbedQuery>;
};

const Embed: React.FC<EmbedProps> = ({ queryRef }) => {
    const { eventById: event } = usePreloadedQuery(query, queryRef);
    const { t } = useTranslation();

    if (!event) {
        return <PlayerPlaceholder>
            <LuFrown />
            <div>{t("not-found.video-not-found")}</div>
        </PlayerPlaceholder>;
    }

    if (event.__typename === "NotAllowed") {
        return <PlayerPlaceholder>
            <LuAlertTriangle />
            <div>{t("api-remote-errors.view.event")}</div>
        </PlayerPlaceholder>;
    }

    if (event.__typename !== "AuthorizedEvent") {
        return unreachable("unhandled event state");
    }

    if (!isSynced(event)) {
        return <PlayerPlaceholder>
            <MovingTruck />
            <div>{t("video.not-ready.title")}</div>
        </PlayerPlaceholder>;
    }

    return <Player event={event} />;
};

export const BlockEmbedRoute = makeRoute({
    match: url => {
        // Only do something if we are embedded
        if (window === window.top) {
            return null;
        }

        // And only if this is a non-embeddable route
        if (url.pathname.startsWith("/~embed/")) {
            return null;
        }

        return {
            render: () => <PlayerPlaceholder>
                <LuAlertTriangle />
                <div>
                    <Translation>{t => t("embed.not-supported")}</Translation>
                </div>
            </PlayerPlaceholder>,
        };
    },
});

class ErrorBoundary extends GlobalErrorBoundary {
    public render(): ReactNode {
        if (!this.state.error) {
            return this.props.children;
        }

        return <PlayerPlaceholder>
            <LuAlertTriangle />
            <div>
                <Translation>{t => t("errors.embedded")}</Translation>
            </div>
        </PlayerPlaceholder>;
    }
}
