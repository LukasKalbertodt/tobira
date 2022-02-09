import React from "react";
import { useTranslation } from "react-i18next";
import { graphql, useFragment, commitLocalUpdate, useRelayEnvironment } from "react-relay";
import type { RecordProxy, RecordSourceProxy } from "relay-runtime";
import {
    FiPlus,
    FiType,
    FiGrid,
    FiFilm,
} from "react-icons/fi";

import { AddButtonsRealmData$key } from "./__generated__/AddButtonsRealmData.graphql";
import { bug } from "../../../../util/err";
import { Button, ButtonGroup } from "./util";


type Props = {
    index: number;
    realm: AddButtonsRealmData$key;
};

export const AddButtons: React.FC<Props> = ({ index, realm }) => {
    const { t } = useTranslation();

    const { id: realmId } = useFragment(graphql`
        fragment AddButtonsRealmData on Realm {
            id
        }
    `, realm);

    const env = useRelayEnvironment();

    const addBlock = (
        type: string,
        prepareBlock?: (store: RecordSourceProxy, block: RecordProxy) => void,
    ) => {
        commitLocalUpdate(env, store => {
            const realm = store.get(realmId) ?? bug("could not find realm");

            const blocks = [
                ...realm.getLinkedRecords("blocks") ?? bug("realm does not have blocks"),
            ];

            const id = "clNEWBLOCK";
            const block = store.create(id, `${type}Block`);
            prepareBlock?.(store, block);
            block.setValue(true, "editMode");
            block.setValue(id, "id");

            blocks.splice(index, 0, block);

            realm.setLinkedRecords(blocks, "blocks");
        });
    };

    return <ButtonGroup css={{ alignSelf: "center" }}>
        <span
            title={t("manage.realm.content.add")}
            css={{
                color: "white",
                backgroundColor: "var(--grey20)",
            }}
        >
            <FiPlus />
        </span>
        <Button title={t("manage.realm.content.add-text")} onClick={() => addBlock("Text")}>
            <FiType />
        </Button>
        <Button
            title={t("manage.realm.content.add-series")}
            onClick={() => addBlock("Series", (store, block) => {
                // This is a horrible hack.
                // The GraphQL schema (and thus the TypeScript types)
                // says that every `SeriesBlock` has a `series: Series!`.
                // We don't. We get away with it by creating a dummy series,
                // which the edit mode of the series blocks knows about.
                const seriesId = "clNOSERIES";
                let dummySeries = store.get(seriesId);
                if (!dummySeries) {
                    dummySeries = store.create(seriesId, "Series");
                    dummySeries.setValue(seriesId, "id");
                }
                block.setLinkedRecord(dummySeries, "series");
                block.setValue("NEW_TO_OLD", "order");
                block.setValue("GRID", "layout");
            })}
        >
            <FiGrid />
        </Button>
        <Button
            title={t("manage.realm.content.add-video")}
            onClick={() => addBlock("Video", (store, block) => {
                // See above for an explanation
                const eventId = "clNOEVENT";
                let dummyEvent = store.get(eventId);
                if (!dummyEvent) {
                    dummyEvent = store.create(eventId, "Event");
                    dummyEvent.setValue(eventId, "id");
                }
                block.setLinkedRecord(dummyEvent, "event");
            })}
        >
            <FiFilm />
        </Button>
    </ButtonGroup>;
};
