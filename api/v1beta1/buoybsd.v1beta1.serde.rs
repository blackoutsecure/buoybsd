impl serde::Serialize for DataWarning {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.field.is_empty() {
            len += 1;
        }
        if !self.reason.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("buoybsd.v1beta1.DataWarning", len)?;
        if !self.field.is_empty() {
            struct_ser.serialize_field("field", &self.field)?;
        }
        if !self.reason.is_empty() {
            struct_ser.serialize_field("reason", &self.reason)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for DataWarning {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "field",
            "reason",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Field,
            Reason,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl serde::de::Visitor<'_> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "field" => Ok(GeneratedField::Field),
                            "reason" => Ok(GeneratedField::Reason),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = DataWarning;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct buoybsd.v1beta1.DataWarning")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<DataWarning, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut field__ = None;
                let mut reason__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Field => {
                            if field__.is_some() {
                                return Err(serde::de::Error::duplicate_field("field"));
                            }
                            field__ = Some(map_.next_value()?);
                        }
                        GeneratedField::Reason => {
                            if reason__.is_some() {
                                return Err(serde::de::Error::duplicate_field("reason"));
                            }
                            reason__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(DataWarning {
                    field: field__.unwrap_or_default(),
                    reason: reason__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("buoybsd.v1beta1.DataWarning", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for Detection {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.id.is_empty() {
            len += 1;
        }
        if !self.name.is_empty() {
            len += 1;
        }
        if !self.description.is_empty() {
            len += 1;
        }
        if !self.severity.is_empty() {
            len += 1;
        }
        if !self.tags.is_empty() {
            len += 1;
        }
        if !self.mitre_techniques.is_empty() {
            len += 1;
        }
        if self.detected_from.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("buoybsd.v1beta1.Detection", len)?;
        if !self.id.is_empty() {
            struct_ser.serialize_field("id", &self.id)?;
        }
        if !self.name.is_empty() {
            struct_ser.serialize_field("name", &self.name)?;
        }
        if !self.description.is_empty() {
            struct_ser.serialize_field("description", &self.description)?;
        }
        if !self.severity.is_empty() {
            struct_ser.serialize_field("severity", &self.severity)?;
        }
        if !self.tags.is_empty() {
            struct_ser.serialize_field("tags", &self.tags)?;
        }
        if !self.mitre_techniques.is_empty() {
            struct_ser.serialize_field("mitre_techniques", &self.mitre_techniques)?;
        }
        if let Some(v) = self.detected_from.as_ref() {
            struct_ser.serialize_field("detected_from", v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for Detection {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "id",
            "name",
            "description",
            "severity",
            "tags",
            "mitre_techniques",
            "mitreTechniques",
            "detected_from",
            "detectedFrom",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Id,
            Name,
            Description,
            Severity,
            Tags,
            MitreTechniques,
            DetectedFrom,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl serde::de::Visitor<'_> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "id" => Ok(GeneratedField::Id),
                            "name" => Ok(GeneratedField::Name),
                            "description" => Ok(GeneratedField::Description),
                            "severity" => Ok(GeneratedField::Severity),
                            "tags" => Ok(GeneratedField::Tags),
                            "mitreTechniques" | "mitre_techniques" => Ok(GeneratedField::MitreTechniques),
                            "detectedFrom" | "detected_from" => Ok(GeneratedField::DetectedFrom),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = Detection;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct buoybsd.v1beta1.Detection")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<Detection, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut id__ = None;
                let mut name__ = None;
                let mut description__ = None;
                let mut severity__ = None;
                let mut tags__ = None;
                let mut mitre_techniques__ = None;
                let mut detected_from__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Id => {
                            if id__.is_some() {
                                return Err(serde::de::Error::duplicate_field("id"));
                            }
                            id__ = Some(map_.next_value()?);
                        }
                        GeneratedField::Name => {
                            if name__.is_some() {
                                return Err(serde::de::Error::duplicate_field("name"));
                            }
                            name__ = Some(map_.next_value()?);
                        }
                        GeneratedField::Description => {
                            if description__.is_some() {
                                return Err(serde::de::Error::duplicate_field("description"));
                            }
                            description__ = Some(map_.next_value()?);
                        }
                        GeneratedField::Severity => {
                            if severity__.is_some() {
                                return Err(serde::de::Error::duplicate_field("severity"));
                            }
                            severity__ = Some(map_.next_value()?);
                        }
                        GeneratedField::Tags => {
                            if tags__.is_some() {
                                return Err(serde::de::Error::duplicate_field("tags"));
                            }
                            tags__ = Some(map_.next_value()?);
                        }
                        GeneratedField::MitreTechniques => {
                            if mitre_techniques__.is_some() {
                                return Err(serde::de::Error::duplicate_field("mitreTechniques"));
                            }
                            mitre_techniques__ = Some(map_.next_value()?);
                        }
                        GeneratedField::DetectedFrom => {
                            if detected_from__.is_some() {
                                return Err(serde::de::Error::duplicate_field("detectedFrom"));
                            }
                            detected_from__ = map_.next_value()?;
                        }
                    }
                }
                Ok(Detection {
                    id: id__.unwrap_or_default(),
                    name: name__.unwrap_or_default(),
                    description: description__.unwrap_or_default(),
                    severity: severity__.unwrap_or_default(),
                    tags: tags__.unwrap_or_default(),
                    mitre_techniques: mitre_techniques__.unwrap_or_default(),
                    detected_from: detected_from__,
                })
            }
        }
        deserializer.deserialize_struct("buoybsd.v1beta1.Detection", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for Event {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.timestamp.is_some() {
            len += 1;
        }
        if self.id != 0 {
            len += 1;
        }
        if !self.name.is_empty() {
            len += 1;
        }
        if !self.schema_version.is_empty() {
            len += 1;
        }
        if !self.record_id.is_empty() {
            len += 1;
        }
        if self.source.is_some() {
            len += 1;
        }
        if self.host.is_some() {
            len += 1;
        }
        if self.process.is_some() {
            len += 1;
        }
        if !self.matched_policies.is_empty() {
            len += 1;
        }
        if !self.detections.is_empty() {
            len += 1;
        }
        if !self.warnings.is_empty() {
            len += 1;
        }
        if self.raw_record.is_some() {
            len += 1;
        }
        if self.jail.is_some() {
            len += 1;
        }
        if self.data.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("buoybsd.v1beta1.Event", len)?;
        if let Some(v) = self.timestamp.as_ref() {
            struct_ser.serialize_field("timestamp", v)?;
        }
        if self.id != 0 {
            let v = EventId::try_from(self.id)
                .map_err(|_| serde::ser::Error::custom(format!("Invalid variant {}", self.id)))?;
            struct_ser.serialize_field("id", &v)?;
        }
        if !self.name.is_empty() {
            struct_ser.serialize_field("name", &self.name)?;
        }
        if !self.schema_version.is_empty() {
            struct_ser.serialize_field("schema_version", &self.schema_version)?;
        }
        if !self.record_id.is_empty() {
            struct_ser.serialize_field("record_id", &self.record_id)?;
        }
        if let Some(v) = self.source.as_ref() {
            struct_ser.serialize_field("source", v)?;
        }
        if let Some(v) = self.host.as_ref() {
            struct_ser.serialize_field("host", v)?;
        }
        if let Some(v) = self.process.as_ref() {
            struct_ser.serialize_field("process", v)?;
        }
        if !self.matched_policies.is_empty() {
            struct_ser.serialize_field("matched_policies", &self.matched_policies)?;
        }
        if !self.detections.is_empty() {
            struct_ser.serialize_field("detections", &self.detections)?;
        }
        if !self.warnings.is_empty() {
            struct_ser.serialize_field("warnings", &self.warnings)?;
        }
        if let Some(v) = self.raw_record.as_ref() {
            struct_ser.serialize_field("raw_record", v)?;
        }
        if let Some(v) = self.jail.as_ref() {
            struct_ser.serialize_field("jail", v)?;
        }
        if let Some(v) = self.data.as_ref() {
            match v {
                event::Data::ProcessExec(v) => {
                    struct_ser.serialize_field("process_exec", v)?;
                }
            }
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for Event {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "timestamp",
            "id",
            "name",
            "schema_version",
            "schemaVersion",
            "record_id",
            "recordId",
            "source",
            "host",
            "process",
            "matched_policies",
            "matchedPolicies",
            "detections",
            "warnings",
            "raw_record",
            "rawRecord",
            "jail",
            "process_exec",
            "processExec",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Timestamp,
            Id,
            Name,
            SchemaVersion,
            RecordId,
            Source,
            Host,
            Process,
            MatchedPolicies,
            Detections,
            Warnings,
            RawRecord,
            Jail,
            ProcessExec,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl serde::de::Visitor<'_> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "timestamp" => Ok(GeneratedField::Timestamp),
                            "id" => Ok(GeneratedField::Id),
                            "name" => Ok(GeneratedField::Name),
                            "schemaVersion" | "schema_version" => Ok(GeneratedField::SchemaVersion),
                            "recordId" | "record_id" => Ok(GeneratedField::RecordId),
                            "source" => Ok(GeneratedField::Source),
                            "host" => Ok(GeneratedField::Host),
                            "process" => Ok(GeneratedField::Process),
                            "matchedPolicies" | "matched_policies" => Ok(GeneratedField::MatchedPolicies),
                            "detections" => Ok(GeneratedField::Detections),
                            "warnings" => Ok(GeneratedField::Warnings),
                            "rawRecord" | "raw_record" => Ok(GeneratedField::RawRecord),
                            "jail" => Ok(GeneratedField::Jail),
                            "processExec" | "process_exec" => Ok(GeneratedField::ProcessExec),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = Event;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct buoybsd.v1beta1.Event")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<Event, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut timestamp__ = None;
                let mut id__ = None;
                let mut name__ = None;
                let mut schema_version__ = None;
                let mut record_id__ = None;
                let mut source__ = None;
                let mut host__ = None;
                let mut process__ = None;
                let mut matched_policies__ = None;
                let mut detections__ = None;
                let mut warnings__ = None;
                let mut raw_record__ = None;
                let mut jail__ = None;
                let mut data__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Timestamp => {
                            if timestamp__.is_some() {
                                return Err(serde::de::Error::duplicate_field("timestamp"));
                            }
                            timestamp__ = map_.next_value()?;
                        }
                        GeneratedField::Id => {
                            if id__.is_some() {
                                return Err(serde::de::Error::duplicate_field("id"));
                            }
                            id__ = Some(map_.next_value::<EventId>()? as i32);
                        }
                        GeneratedField::Name => {
                            if name__.is_some() {
                                return Err(serde::de::Error::duplicate_field("name"));
                            }
                            name__ = Some(map_.next_value()?);
                        }
                        GeneratedField::SchemaVersion => {
                            if schema_version__.is_some() {
                                return Err(serde::de::Error::duplicate_field("schemaVersion"));
                            }
                            schema_version__ = Some(map_.next_value()?);
                        }
                        GeneratedField::RecordId => {
                            if record_id__.is_some() {
                                return Err(serde::de::Error::duplicate_field("recordId"));
                            }
                            record_id__ = Some(map_.next_value()?);
                        }
                        GeneratedField::Source => {
                            if source__.is_some() {
                                return Err(serde::de::Error::duplicate_field("source"));
                            }
                            source__ = map_.next_value()?;
                        }
                        GeneratedField::Host => {
                            if host__.is_some() {
                                return Err(serde::de::Error::duplicate_field("host"));
                            }
                            host__ = map_.next_value()?;
                        }
                        GeneratedField::Process => {
                            if process__.is_some() {
                                return Err(serde::de::Error::duplicate_field("process"));
                            }
                            process__ = map_.next_value()?;
                        }
                        GeneratedField::MatchedPolicies => {
                            if matched_policies__.is_some() {
                                return Err(serde::de::Error::duplicate_field("matchedPolicies"));
                            }
                            matched_policies__ = Some(map_.next_value()?);
                        }
                        GeneratedField::Detections => {
                            if detections__.is_some() {
                                return Err(serde::de::Error::duplicate_field("detections"));
                            }
                            detections__ = Some(map_.next_value()?);
                        }
                        GeneratedField::Warnings => {
                            if warnings__.is_some() {
                                return Err(serde::de::Error::duplicate_field("warnings"));
                            }
                            warnings__ = Some(map_.next_value()?);
                        }
                        GeneratedField::RawRecord => {
                            if raw_record__.is_some() {
                                return Err(serde::de::Error::duplicate_field("rawRecord"));
                            }
                            raw_record__ = map_.next_value()?;
                        }
                        GeneratedField::Jail => {
                            if jail__.is_some() {
                                return Err(serde::de::Error::duplicate_field("jail"));
                            }
                            jail__ = map_.next_value()?;
                        }
                        GeneratedField::ProcessExec => {
                            if data__.is_some() {
                                return Err(serde::de::Error::duplicate_field("processExec"));
                            }
                            data__ = map_.next_value::<::std::option::Option<_>>()?.map(event::Data::ProcessExec)
;
                        }
                    }
                }
                Ok(Event {
                    timestamp: timestamp__,
                    id: id__.unwrap_or_default(),
                    name: name__.unwrap_or_default(),
                    schema_version: schema_version__.unwrap_or_default(),
                    record_id: record_id__.unwrap_or_default(),
                    source: source__,
                    host: host__,
                    process: process__,
                    matched_policies: matched_policies__.unwrap_or_default(),
                    detections: detections__.unwrap_or_default(),
                    warnings: warnings__.unwrap_or_default(),
                    raw_record: raw_record__,
                    jail: jail__,
                    data: data__,
                })
            }
        }
        deserializer.deserialize_struct("buoybsd.v1beta1.Event", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for EventId {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let variant = match self {
            Self::Unspecified => "EVENT_ID_UNSPECIFIED",
            Self::ProcessExec => "EVENT_ID_PROCESS_EXEC",
        };
        serializer.serialize_str(variant)
    }
}
impl<'de> serde::Deserialize<'de> for EventId {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "EVENT_ID_UNSPECIFIED",
            "EVENT_ID_PROCESS_EXEC",
        ];

        struct GeneratedVisitor;

        impl serde::de::Visitor<'_> for GeneratedVisitor {
            type Value = EventId;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(formatter, "expected one of: {:?}", FIELDS)
            }

            fn visit_i64<E>(self, v: i64) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                i32::try_from(v)
                    .ok()
                    .and_then(|x| x.try_into().ok())
                    .ok_or_else(|| {
                        serde::de::Error::invalid_value(serde::de::Unexpected::Signed(v), &self)
                    })
            }

            fn visit_u64<E>(self, v: u64) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                i32::try_from(v)
                    .ok()
                    .and_then(|x| x.try_into().ok())
                    .ok_or_else(|| {
                        serde::de::Error::invalid_value(serde::de::Unexpected::Unsigned(v), &self)
                    })
            }

            fn visit_str<E>(self, value: &str) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                match value {
                    "EVENT_ID_UNSPECIFIED" => Ok(EventId::Unspecified),
                    "EVENT_ID_PROCESS_EXEC" => Ok(EventId::ProcessExec),
                    _ => Err(serde::de::Error::unknown_variant(value, FIELDS)),
                }
            }
        }
        deserializer.deserialize_any(GeneratedVisitor)
    }
}
impl serde::Serialize for EventReference {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.id != 0 {
            len += 1;
        }
        if !self.name.is_empty() {
            len += 1;
        }
        if !self.record_id.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("buoybsd.v1beta1.EventReference", len)?;
        if self.id != 0 {
            let v = EventId::try_from(self.id)
                .map_err(|_| serde::ser::Error::custom(format!("Invalid variant {}", self.id)))?;
            struct_ser.serialize_field("id", &v)?;
        }
        if !self.name.is_empty() {
            struct_ser.serialize_field("name", &self.name)?;
        }
        if !self.record_id.is_empty() {
            struct_ser.serialize_field("record_id", &self.record_id)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for EventReference {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "id",
            "name",
            "record_id",
            "recordId",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Id,
            Name,
            RecordId,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl serde::de::Visitor<'_> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "id" => Ok(GeneratedField::Id),
                            "name" => Ok(GeneratedField::Name),
                            "recordId" | "record_id" => Ok(GeneratedField::RecordId),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = EventReference;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct buoybsd.v1beta1.EventReference")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<EventReference, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut id__ = None;
                let mut name__ = None;
                let mut record_id__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Id => {
                            if id__.is_some() {
                                return Err(serde::de::Error::duplicate_field("id"));
                            }
                            id__ = Some(map_.next_value::<EventId>()? as i32);
                        }
                        GeneratedField::Name => {
                            if name__.is_some() {
                                return Err(serde::de::Error::duplicate_field("name"));
                            }
                            name__ = Some(map_.next_value()?);
                        }
                        GeneratedField::RecordId => {
                            if record_id__.is_some() {
                                return Err(serde::de::Error::duplicate_field("recordId"));
                            }
                            record_id__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(EventReference {
                    id: id__.unwrap_or_default(),
                    name: name__.unwrap_or_default(),
                    record_id: record_id__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("buoybsd.v1beta1.EventReference", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for Host {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.name.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("buoybsd.v1beta1.Host", len)?;
        if let Some(v) = self.name.as_ref() {
            struct_ser.serialize_field("name", v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for Host {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "name",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Name,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl serde::de::Visitor<'_> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "name" => Ok(GeneratedField::Name),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = Host;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct buoybsd.v1beta1.Host")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<Host, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut name__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Name => {
                            if name__.is_some() {
                                return Err(serde::de::Error::duplicate_field("name"));
                            }
                            name__ = map_.next_value()?;
                        }
                    }
                }
                Ok(Host {
                    name: name__,
                })
            }
        }
        deserializer.deserialize_struct("buoybsd.v1beta1.Host", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for Jail {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.id.is_some() {
            len += 1;
        }
        if self.name.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("buoybsd.v1beta1.Jail", len)?;
        if let Some(v) = self.id.as_ref() {
            struct_ser.serialize_field("id", v)?;
        }
        if let Some(v) = self.name.as_ref() {
            struct_ser.serialize_field("name", v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for Jail {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "id",
            "name",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Id,
            Name,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl serde::de::Visitor<'_> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "id" => Ok(GeneratedField::Id),
                            "name" => Ok(GeneratedField::Name),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = Jail;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct buoybsd.v1beta1.Jail")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<Jail, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut id__ = None;
                let mut name__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Id => {
                            if id__.is_some() {
                                return Err(serde::de::Error::duplicate_field("id"));
                            }
                            id__ = 
                                map_.next_value::<::std::option::Option<::pbjson::private::NumberDeserialize<_>>>()?.map(|x| x.0)
                            ;
                        }
                        GeneratedField::Name => {
                            if name__.is_some() {
                                return Err(serde::de::Error::duplicate_field("name"));
                            }
                            name__ = map_.next_value()?;
                        }
                    }
                }
                Ok(Jail {
                    id: id__,
                    name: name__,
                })
            }
        }
        deserializer.deserialize_struct("buoybsd.v1beta1.Jail", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for Process {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.pid.is_some() {
            len += 1;
        }
        if self.tid.is_some() {
            len += 1;
        }
        if self.parent_pid.is_some() {
            len += 1;
        }
        if self.uid.is_some() {
            len += 1;
        }
        if self.euid.is_some() {
            len += 1;
        }
        if self.gid.is_some() {
            len += 1;
        }
        if self.egid.is_some() {
            len += 1;
        }
        if self.executable_path.is_some() {
            len += 1;
        }
        if self.command.is_some() {
            len += 1;
        }
        if !self.ancestors.is_empty() {
            len += 1;
        }
        if self.ancestry_complete.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("buoybsd.v1beta1.Process", len)?;
        if let Some(v) = self.pid.as_ref() {
            struct_ser.serialize_field("pid", v)?;
        }
        if let Some(v) = self.tid.as_ref() {
            struct_ser.serialize_field("tid", v)?;
        }
        if let Some(v) = self.parent_pid.as_ref() {
            struct_ser.serialize_field("parent_pid", v)?;
        }
        if let Some(v) = self.uid.as_ref() {
            struct_ser.serialize_field("uid", v)?;
        }
        if let Some(v) = self.euid.as_ref() {
            struct_ser.serialize_field("euid", v)?;
        }
        if let Some(v) = self.gid.as_ref() {
            struct_ser.serialize_field("gid", v)?;
        }
        if let Some(v) = self.egid.as_ref() {
            struct_ser.serialize_field("egid", v)?;
        }
        if let Some(v) = self.executable_path.as_ref() {
            struct_ser.serialize_field("executable_path", v)?;
        }
        if let Some(v) = self.command.as_ref() {
            struct_ser.serialize_field("command", v)?;
        }
        if !self.ancestors.is_empty() {
            struct_ser.serialize_field("ancestors", &self.ancestors)?;
        }
        if let Some(v) = self.ancestry_complete.as_ref() {
            struct_ser.serialize_field("ancestry_complete", v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for Process {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "pid",
            "tid",
            "parent_pid",
            "parentPid",
            "uid",
            "euid",
            "gid",
            "egid",
            "executable_path",
            "executablePath",
            "command",
            "ancestors",
            "ancestry_complete",
            "ancestryComplete",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Pid,
            Tid,
            ParentPid,
            Uid,
            Euid,
            Gid,
            Egid,
            ExecutablePath,
            Command,
            Ancestors,
            AncestryComplete,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl serde::de::Visitor<'_> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "pid" => Ok(GeneratedField::Pid),
                            "tid" => Ok(GeneratedField::Tid),
                            "parentPid" | "parent_pid" => Ok(GeneratedField::ParentPid),
                            "uid" => Ok(GeneratedField::Uid),
                            "euid" => Ok(GeneratedField::Euid),
                            "gid" => Ok(GeneratedField::Gid),
                            "egid" => Ok(GeneratedField::Egid),
                            "executablePath" | "executable_path" => Ok(GeneratedField::ExecutablePath),
                            "command" => Ok(GeneratedField::Command),
                            "ancestors" => Ok(GeneratedField::Ancestors),
                            "ancestryComplete" | "ancestry_complete" => Ok(GeneratedField::AncestryComplete),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = Process;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct buoybsd.v1beta1.Process")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<Process, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut pid__ = None;
                let mut tid__ = None;
                let mut parent_pid__ = None;
                let mut uid__ = None;
                let mut euid__ = None;
                let mut gid__ = None;
                let mut egid__ = None;
                let mut executable_path__ = None;
                let mut command__ = None;
                let mut ancestors__ = None;
                let mut ancestry_complete__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Pid => {
                            if pid__.is_some() {
                                return Err(serde::de::Error::duplicate_field("pid"));
                            }
                            pid__ = 
                                map_.next_value::<::std::option::Option<::pbjson::private::NumberDeserialize<_>>>()?.map(|x| x.0)
                            ;
                        }
                        GeneratedField::Tid => {
                            if tid__.is_some() {
                                return Err(serde::de::Error::duplicate_field("tid"));
                            }
                            tid__ = 
                                map_.next_value::<::std::option::Option<::pbjson::private::NumberDeserialize<_>>>()?.map(|x| x.0)
                            ;
                        }
                        GeneratedField::ParentPid => {
                            if parent_pid__.is_some() {
                                return Err(serde::de::Error::duplicate_field("parentPid"));
                            }
                            parent_pid__ = 
                                map_.next_value::<::std::option::Option<::pbjson::private::NumberDeserialize<_>>>()?.map(|x| x.0)
                            ;
                        }
                        GeneratedField::Uid => {
                            if uid__.is_some() {
                                return Err(serde::de::Error::duplicate_field("uid"));
                            }
                            uid__ = 
                                map_.next_value::<::std::option::Option<::pbjson::private::NumberDeserialize<_>>>()?.map(|x| x.0)
                            ;
                        }
                        GeneratedField::Euid => {
                            if euid__.is_some() {
                                return Err(serde::de::Error::duplicate_field("euid"));
                            }
                            euid__ = 
                                map_.next_value::<::std::option::Option<::pbjson::private::NumberDeserialize<_>>>()?.map(|x| x.0)
                            ;
                        }
                        GeneratedField::Gid => {
                            if gid__.is_some() {
                                return Err(serde::de::Error::duplicate_field("gid"));
                            }
                            gid__ = 
                                map_.next_value::<::std::option::Option<::pbjson::private::NumberDeserialize<_>>>()?.map(|x| x.0)
                            ;
                        }
                        GeneratedField::Egid => {
                            if egid__.is_some() {
                                return Err(serde::de::Error::duplicate_field("egid"));
                            }
                            egid__ = 
                                map_.next_value::<::std::option::Option<::pbjson::private::NumberDeserialize<_>>>()?.map(|x| x.0)
                            ;
                        }
                        GeneratedField::ExecutablePath => {
                            if executable_path__.is_some() {
                                return Err(serde::de::Error::duplicate_field("executablePath"));
                            }
                            executable_path__ = map_.next_value()?;
                        }
                        GeneratedField::Command => {
                            if command__.is_some() {
                                return Err(serde::de::Error::duplicate_field("command"));
                            }
                            command__ = map_.next_value()?;
                        }
                        GeneratedField::Ancestors => {
                            if ancestors__.is_some() {
                                return Err(serde::de::Error::duplicate_field("ancestors"));
                            }
                            ancestors__ = Some(map_.next_value()?);
                        }
                        GeneratedField::AncestryComplete => {
                            if ancestry_complete__.is_some() {
                                return Err(serde::de::Error::duplicate_field("ancestryComplete"));
                            }
                            ancestry_complete__ = map_.next_value()?;
                        }
                    }
                }
                Ok(Process {
                    pid: pid__,
                    tid: tid__,
                    parent_pid: parent_pid__,
                    uid: uid__,
                    euid: euid__,
                    gid: gid__,
                    egid: egid__,
                    executable_path: executable_path__,
                    command: command__,
                    ancestors: ancestors__.unwrap_or_default(),
                    ancestry_complete: ancestry_complete__,
                })
            }
        }
        deserializer.deserialize_struct("buoybsd.v1beta1.Process", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for ProcessExec {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.argv.is_empty() {
            len += 1;
        }
        if self.argv_complete.is_some() {
            len += 1;
        }
        if self.success.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("buoybsd.v1beta1.ProcessExec", len)?;
        if !self.argv.is_empty() {
            struct_ser.serialize_field("argv", &self.argv)?;
        }
        if let Some(v) = self.argv_complete.as_ref() {
            struct_ser.serialize_field("argv_complete", v)?;
        }
        if let Some(v) = self.success.as_ref() {
            struct_ser.serialize_field("success", v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for ProcessExec {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "argv",
            "argv_complete",
            "argvComplete",
            "success",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Argv,
            ArgvComplete,
            Success,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl serde::de::Visitor<'_> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "argv" => Ok(GeneratedField::Argv),
                            "argvComplete" | "argv_complete" => Ok(GeneratedField::ArgvComplete),
                            "success" => Ok(GeneratedField::Success),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = ProcessExec;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct buoybsd.v1beta1.ProcessExec")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<ProcessExec, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut argv__ = None;
                let mut argv_complete__ = None;
                let mut success__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Argv => {
                            if argv__.is_some() {
                                return Err(serde::de::Error::duplicate_field("argv"));
                            }
                            argv__ = Some(map_.next_value()?);
                        }
                        GeneratedField::ArgvComplete => {
                            if argv_complete__.is_some() {
                                return Err(serde::de::Error::duplicate_field("argvComplete"));
                            }
                            argv_complete__ = map_.next_value()?;
                        }
                        GeneratedField::Success => {
                            if success__.is_some() {
                                return Err(serde::de::Error::duplicate_field("success"));
                            }
                            success__ = map_.next_value()?;
                        }
                    }
                }
                Ok(ProcessExec {
                    argv: argv__.unwrap_or_default(),
                    argv_complete: argv_complete__,
                    success: success__,
                })
            }
        }
        deserializer.deserialize_struct("buoybsd.v1beta1.ProcessExec", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for RawRecord {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.media_type.is_empty() {
            len += 1;
        }
        if !self.data.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("buoybsd.v1beta1.RawRecord", len)?;
        if !self.media_type.is_empty() {
            struct_ser.serialize_field("media_type", &self.media_type)?;
        }
        if !self.data.is_empty() {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("data", pbjson::private::base64::encode(&self.data).as_str())?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for RawRecord {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "media_type",
            "mediaType",
            "data",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            MediaType,
            Data,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl serde::de::Visitor<'_> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "mediaType" | "media_type" => Ok(GeneratedField::MediaType),
                            "data" => Ok(GeneratedField::Data),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = RawRecord;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct buoybsd.v1beta1.RawRecord")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<RawRecord, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut media_type__ = None;
                let mut data__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::MediaType => {
                            if media_type__.is_some() {
                                return Err(serde::de::Error::duplicate_field("mediaType"));
                            }
                            media_type__ = Some(map_.next_value()?);
                        }
                        GeneratedField::Data => {
                            if data__.is_some() {
                                return Err(serde::de::Error::duplicate_field("data"));
                            }
                            data__ = 
                                Some(map_.next_value::<::pbjson::private::BytesDeserialize<_>>()?.0)
                            ;
                        }
                    }
                }
                Ok(RawRecord {
                    media_type: media_type__.unwrap_or_default(),
                    data: data__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("buoybsd.v1beta1.RawRecord", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for Source {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.collector.is_empty() {
            len += 1;
        }
        if !self.operating_system.is_empty() {
            len += 1;
        }
        if !self.evidence.is_empty() {
            len += 1;
        }
        if self.source_id.is_some() {
            len += 1;
        }
        if self.replay_sequence != 0 {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("buoybsd.v1beta1.Source", len)?;
        if !self.collector.is_empty() {
            struct_ser.serialize_field("collector", &self.collector)?;
        }
        if !self.operating_system.is_empty() {
            struct_ser.serialize_field("operating_system", &self.operating_system)?;
        }
        if !self.evidence.is_empty() {
            struct_ser.serialize_field("evidence", &self.evidence)?;
        }
        if let Some(v) = self.source_id.as_ref() {
            struct_ser.serialize_field("source_id", v)?;
        }
        if self.replay_sequence != 0 {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("replay_sequence", ToString::to_string(&self.replay_sequence).as_str())?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for Source {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "collector",
            "operating_system",
            "operatingSystem",
            "evidence",
            "source_id",
            "sourceId",
            "replay_sequence",
            "replaySequence",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Collector,
            OperatingSystem,
            Evidence,
            SourceId,
            ReplaySequence,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl serde::de::Visitor<'_> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "collector" => Ok(GeneratedField::Collector),
                            "operatingSystem" | "operating_system" => Ok(GeneratedField::OperatingSystem),
                            "evidence" => Ok(GeneratedField::Evidence),
                            "sourceId" | "source_id" => Ok(GeneratedField::SourceId),
                            "replaySequence" | "replay_sequence" => Ok(GeneratedField::ReplaySequence),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = Source;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct buoybsd.v1beta1.Source")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<Source, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut collector__ = None;
                let mut operating_system__ = None;
                let mut evidence__ = None;
                let mut source_id__ = None;
                let mut replay_sequence__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Collector => {
                            if collector__.is_some() {
                                return Err(serde::de::Error::duplicate_field("collector"));
                            }
                            collector__ = Some(map_.next_value()?);
                        }
                        GeneratedField::OperatingSystem => {
                            if operating_system__.is_some() {
                                return Err(serde::de::Error::duplicate_field("operatingSystem"));
                            }
                            operating_system__ = Some(map_.next_value()?);
                        }
                        GeneratedField::Evidence => {
                            if evidence__.is_some() {
                                return Err(serde::de::Error::duplicate_field("evidence"));
                            }
                            evidence__ = Some(map_.next_value()?);
                        }
                        GeneratedField::SourceId => {
                            if source_id__.is_some() {
                                return Err(serde::de::Error::duplicate_field("sourceId"));
                            }
                            source_id__ = map_.next_value()?;
                        }
                        GeneratedField::ReplaySequence => {
                            if replay_sequence__.is_some() {
                                return Err(serde::de::Error::duplicate_field("replaySequence"));
                            }
                            replay_sequence__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                    }
                }
                Ok(Source {
                    collector: collector__.unwrap_or_default(),
                    operating_system: operating_system__.unwrap_or_default(),
                    evidence: evidence__.unwrap_or_default(),
                    source_id: source_id__,
                    replay_sequence: replay_sequence__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("buoybsd.v1beta1.Source", FIELDS, GeneratedVisitor)
    }
}
