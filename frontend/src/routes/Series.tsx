import { graphql } from "react-relay";
import { useTranslation } from "react-i18next";

import { discriminate } from "../util";
import { unreachable } from "../util/err";
import { loadQuery } from "../relay";
import { makeRoute } from "../rauta";
import { ErrorPage } from "../ui/error";
import { SeriesBlockFromSeries } from "../ui/Blocks/Series";
import { RootLoader } from "../layout/Root";
import { Nav } from "../layout/Navigation";
import { PageTitle } from "../layout/header/ui";

import { NotFound } from "./NotFound";
import {
    SeriesByOpencastIdQuery,
    SeriesByOpencastIdQuery$data,
} from "./__generated__/SeriesByOpencastIdQuery.graphql";


export const DirectSeriesRoute = makeRoute(url => {

    const regex = new RegExp("^/!s/:([^/]+)$", "u");
    const matches = regex.exec(url.pathname);

    if (!matches) {
        return null;
    }


    const opencastId = decodeURIComponent(matches[1]);
    const query = graphql`
        query SeriesByOpencastIdQuery($id: String!) {
            ... UserData
            series: seriesByOpencastId(id: $id) {
                __typename
                ... on WaitingSeries { id } # only for correct type generation ...
                ... on ReadySeries {
                    title
                    description
                    ... SeriesBlockSeriesData
                }
            }
            rootRealm {
               ... NavigationData
            }
        }
    `;
    const queryRef = loadQuery<SeriesByOpencastIdQuery>(query, { id: opencastId });


    return {
        render: () => <RootLoader
            {...{ query, queryRef }}
            nav={data => <Nav fragRef={data.rootRealm} />}
            render={result => <SeriesPage {...result} />}
        />,
        dispose: () => queryRef.dispose(),
    };
});

const SeriesPage: React.FC<SeriesByOpencastIdQuery$data> = ({ series }) => {
    const { t } = useTranslation();

    if (!series) {
        return <NotFound kind="series" />;
    }

    return discriminate(series, "__typename", {
        WaitingSeries: () => (
            <ErrorPage title={t("series.not-ready.title")}>
                {t("series.not-ready.text")}
            </ErrorPage>
        ),
        ReadySeries: series => (
            <div css={{ display: "flex", flexDirection: "column" }}>
                <PageTitle title={series.title} />
                <p>{series.description}</p>
                <div css={{ marginTop: 12 }}>
                    <SeriesBlockFromSeries
                        title={t("series.videos.heading")}
                        basePath="/!v"
                        fragRef={series}
                    />
                </div>
            </div>
        ),
    }, () => unreachable());
};