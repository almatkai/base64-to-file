var $c950f3189e9100ee$export$cd58ffd7e3880e66 = /*#__PURE__*/ function(FormValidation) {
    /** Show an error when the value of the item is empty */ FormValidation["Required"] = "required";
    return FormValidation;
}({});
function $c950f3189e9100ee$var$validationError(validation, value) {
    if (validation) {
        if (typeof validation === "function") return validation(value);
        else if (validation === "required") {
            let valueIsValid = typeof value !== "undefined" && value !== null;
            if (valueIsValid) switch(typeof value){
                case "string":
                    valueIsValid = value.length > 0;
                    break;
                case "object":
                    if (Array.isArray(value)) valueIsValid = value.length > 0;
                    else if (value instanceof Date) valueIsValid = value.getTime() > 0;
                    break;
                default:
                    break;
            }
            if (!valueIsValid) return "The item is required";
        }
    }
}
function $c950f3189e9100ee$export$87c0cf8eb5a167e0(props) {
    const { onSubmit: _onSubmit, validation: validation, initialValues: initialValues = {} } = props;
    // @ts-expect-error it's fine if we don't specify all the values
    const [values, setValues] = (0, $8zHUo$react.useState)(initialValues);
    const [errors, setErrors] = (0, $8zHUo$react.useState)({});
    const refs = (0, $8zHUo$react.useRef)({});
    const latestValidation = (0, $07a180af23fbf38b$export$d4b699e2c1148419)(validation || {});
    const latestOnSubmit = (0, $07a180af23fbf38b$export$d4b699e2c1148419)(_onSubmit);
    const focus = (0, $8zHUo$react.useCallback)((id)=>{
        refs.current[id]?.focus();
    }, [
        refs
    ]);
    const handleSubmit = (0, $8zHUo$react.useCallback)(async (values)=>{
        let validationErrors = false;
        for (const [id, validation] of Object.entries(latestValidation.current)){
            const error = $c950f3189e9100ee$var$validationError(validation, values[id]);
            if (error) {
                if (!validationErrors) {
                    validationErrors = {};
                    // we focus the first item that has an error
                    focus(id);
                }
                validationErrors[id] = error;
            }
        }
        if (validationErrors) {
            setErrors(validationErrors);
            return false;
        }
        const result = await latestOnSubmit.current(values);
        return typeof result === "boolean" ? result : true;
    }, [
        latestValidation,
        latestOnSubmit,
        focus
    ]);
    const setValidationError = (0, $8zHUo$react.useCallback)((id, error)=>{
        setErrors((errors)=>({
                ...errors,
                [id]: error
            }));
    }, [
        setErrors
    ]);
    const setValue = (0, $8zHUo$react.useCallback)(function(id, value) {
        // @ts-expect-error TS is always confused about SetStateAction, but it's fine here
        setValues((values)=>({
                ...values,
                [id]: typeof value === "function" ? value(values[id]) : value
            }));
    }, [
        setValues
    ]);
    const itemProps = (0, $8zHUo$react.useMemo)(()=>{
        // we have to use a proxy because we don't actually have any object to iterate through
        // so instead we dynamically create the props when required
        return new Proxy(// @ts-expect-error the whole point of a proxy...
        {}, {
            get (target, id) {
                const validation = latestValidation.current[id];
                const value = values[id];
                return {
                    onChange (value) {
                        if (errors[id]) {
                            const error = $c950f3189e9100ee$var$validationError(validation, value);
                            if (!error) setValidationError(id, undefined);
                        }
                        setValue(id, value);
                    },
                    onBlur (event) {
                        const error = $c950f3189e9100ee$var$validationError(validation, event.target.value);
                        if (error) setValidationError(id, error);
                    },
                    error: errors[id],
                    id: id,
                    // we shouldn't return `undefined` otherwise it will be an uncontrolled component
                    value: typeof value === "undefined" ? null : value,
                    ref: (instance)=>{
                        refs.current[id] = instance;
                    }
                };
            }
        });
    }, [
        errors,
        latestValidation,
        setValidationError,
        values,
        refs,
        setValue
    ]);
    const reset = (0, $8zHUo$react.useCallback)((values)=>{
        setErrors({});
        Object.entries(refs.current).forEach(([id, ref])=>{
            if (!values || !Object.prototype.hasOwnProperty.call(values, id)) ref?.reset();
        });
        if (values) // @ts-expect-error it's fine if we don't specify all the values
        setValues(values);
    }, [
        setValues,
        setErrors,
        refs
    ]);
    return {
        handleSubmit: handleSubmit,
        setValidationError: setValidationError,
        setValue: setValue,
        values: values,
        itemProps: itemProps,
        focus: focus,
        reset: reset
    };
}





