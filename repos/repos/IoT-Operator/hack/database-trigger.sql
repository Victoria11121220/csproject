-- Create a trigger function
CREATE OR REPLACE FUNCTION notify_iot_flow_change() RETURNS TRIGGER AS $$
BEGIN
    IF TG_OP = 'INSERT' OR TG_OP = 'UPDATE' OR TG_OP = 'DELETE' THEN
        PERFORM pg_notify('iot_flow_change', TG_OP || ':' || COALESCE(NEW.id::text, OLD.id::text));
        RETURN NEW;
    END IF;
    RETURN NULL;
END;
$$ LANGUAGE plpgsql;

-- Create a trigger for the iot_flow table
DROP TRIGGER IF EXISTS iot_flow_change_trigger ON iot_flow;
CREATE TRIGGER iot_flow_change_trigger
AFTER INSERT OR UPDATE OR DELETE ON iot_flow
FOR EACH ROW EXECUTE FUNCTION notify_iot_flow_change();